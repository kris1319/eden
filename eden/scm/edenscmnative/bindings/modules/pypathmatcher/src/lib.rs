/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#![allow(non_camel_case_types)]

use std::path::Path;
use std::sync::Arc;

use cpython::*;
use cpython_ext::error::{AnyhowResultExt, ResultPyErrExt};
use cpython_ext::{ExtractInner, ExtractInnerRef, PyPath, PyPathBuf, Str};

use anyhow::Result;
use pathmatcher::{
    AlwaysMatcher, DifferenceMatcher, DirectoryMatch, GitignoreMatcher, Matcher, TreeMatcher,
    UnionMatcher,
};
use types::RepoPath;

pub fn init_module(py: Python, package: &str) -> PyResult<PyModule> {
    let name = [package, "pathmatcher"].join(".");
    let m = PyModule::new(py, &name)?;
    m.add_class::<gitignorematcher>(py)?;
    m.add_class::<treematcher>(py)?;
    m.add(py, "normalizeglob", py_fn!(py, normalize_glob(path: &str)))?;
    m.add(py, "plaintoglob", py_fn!(py, plain_to_glob(path: &str)))?;
    m.add(
        py,
        "expandcurlybrackets",
        py_fn!(py, expand_curly_brackets(path: &str)),
    )?;
    Ok(m)
}

py_class!(class gitignorematcher |py| {
    data matcher: GitignoreMatcher;

    def __new__(_cls, root: &PyPath, global_paths: Vec<PyPathBuf>) -> PyResult<gitignorematcher> {
        let global_paths: Vec<&Path> = global_paths.iter().map(PyPathBuf::as_path).collect();
        let matcher = GitignoreMatcher::new(root, global_paths);
        gitignorematcher::create_instance(py, matcher)
    }

    def match_relative(&self, path: &PyPath, is_dir: bool) -> PyResult<bool> {
        Ok(self.matcher(py).match_relative(path, is_dir))
    }

    def explain(&self, path: &PyPath, is_dir: bool) -> PyResult<Str> {
        Ok(self.matcher(py).explain(path, is_dir).into())
    }
});

py_class!(pub class treematcher |py| {
    data matcher: Arc<TreeMatcher>;

    def __new__(_cls, rules: Vec<String>) -> PyResult<Self> {
        let matcher = TreeMatcher::from_rules(rules.into_iter()).map_pyerr(py)?;
        Self::create_instance(py, Arc::new(matcher))
    }

    def matches(&self, path: &PyPath) -> PyResult<bool> {
        Ok(self.matcher(py).matches(path))
    }

    def match_recursive(&self, path: &PyPath) -> PyResult<Option<bool>> {
        if path.as_path().as_os_str().is_empty() {
            Ok(None)
        } else {
            Ok(self.matcher(py).match_recursive(path))
        }
    }
});

impl ExtractInnerRef for treematcher {
    type Inner = Arc<TreeMatcher>;

    fn extract_inner_ref<'a>(&'a self, py: Python<'a>) -> &'a Self::Inner {
        self.matcher(py)
    }
}

fn normalize_glob(_py: Python, path: &str) -> PyResult<Str> {
    Ok(pathmatcher::normalize_glob(path).into())
}

fn plain_to_glob(_py: Python, path: &str) -> PyResult<Str> {
    Ok(pathmatcher::plain_to_glob(path).into())
}

fn expand_curly_brackets(_py: Python, pattern: &str) -> PyResult<Vec<Str>> {
    Ok(pathmatcher::expand_curly_brackets(pattern)
        .into_iter()
        .map(|s| s.into())
        .collect())
}

pub struct PythonMatcher<'a> {
    py: Python<'a>,
    py_matcher: PyObject,
}

impl<'a> PythonMatcher<'a> {
    pub fn new(py: Python<'a>, py_matcher: PyObject) -> Self {
        PythonMatcher { py, py_matcher }
    }
}

impl<'a> Matcher for PythonMatcher<'a> {
    fn matches_directory(&self, path: &RepoPath) -> Result<DirectoryMatch> {
        matches_directory_impl(self.py, &self.py_matcher, &path).into_anyhow_result()
    }

    fn matches_file(&self, path: &RepoPath) -> Result<bool> {
        matches_file_impl(self.py, &self.py_matcher, &path).into_anyhow_result()
    }
}

pub struct ThreadPythonMatcher {
    py_matcher: PyObject,
}

impl ThreadPythonMatcher {
    pub fn new(py_matcher: PyObject) -> Self {
        ThreadPythonMatcher { py_matcher }
    }
}

impl Matcher for ThreadPythonMatcher {
    fn matches_directory(&self, path: &RepoPath) -> Result<DirectoryMatch> {
        let gil = Python::acquire_gil();
        matches_directory_impl(gil.python(), &self.py_matcher, &path).into_anyhow_result()
    }

    fn matches_file(&self, path: &RepoPath) -> Result<bool> {
        let gil = Python::acquire_gil();
        matches_file_impl(gil.python(), &self.py_matcher, &path).into_anyhow_result()
    }
}

// Matcher which does not store py. Should only be used when py cannot be stored in PythonMatcher
// struct and it is known that the GIL is acquired when calling matcher methods.
// Otherwise use PythonMatcher struct above
pub struct UnsafePythonMatcher {
    py_matcher: PyObject,
}

impl UnsafePythonMatcher {
    pub fn new(py_matcher: PyObject) -> Self {
        UnsafePythonMatcher { py_matcher }
    }
}

impl Clone for UnsafePythonMatcher {
    fn clone(&self) -> Self {
        let gil = Python::acquire_gil();
        UnsafePythonMatcher {
            py_matcher: self.py_matcher.clone_ref(gil.python()),
        }
    }
}

impl<'a> Matcher for UnsafePythonMatcher {
    fn matches_directory(&self, path: &RepoPath) -> Result<DirectoryMatch> {
        let assumed_py = unsafe { Python::assume_gil_acquired() };
        matches_directory_impl(assumed_py, &self.py_matcher, &path).into_anyhow_result()
    }

    fn matches_file(&self, path: &RepoPath) -> Result<bool> {
        let assumed_py = unsafe { Python::assume_gil_acquired() };
        matches_file_impl(assumed_py, &self.py_matcher, &path).into_anyhow_result()
    }
}

fn matches_directory_impl(
    py: Python,
    py_matcher: &PyObject,
    path: &RepoPath,
) -> PyResult<DirectoryMatch> {
    let py_path = PyPathBuf::from(path);
    // PANICS! The interface in Rust doesn't expose exceptions. Unwrapping seems fine since
    // it crashes the rust stuff and returns a rust exception to Python.
    let py_value = py_matcher.call_method(py, "visitdir", (py_path,), None)?;

    let is_all = PyString::extract(py, &py_value)
        .and_then(|py_str| py_str.to_string(py).map(|s| s == "all"))
        .unwrap_or(false);
    let matches = if is_all {
        DirectoryMatch::Everything
    } else {
        if py_value.is_true(py).unwrap() {
            DirectoryMatch::ShouldTraverse
        } else {
            DirectoryMatch::Nothing
        }
    };
    Ok(matches)
}

fn matches_file_impl(py: Python, py_matcher: &PyObject, path: &RepoPath) -> PyResult<bool> {
    let py_path = PyPathBuf::from(path);
    // PANICS! The interface in Rust doesn't expose exceptions. Unwrapping seems fine since
    // it crashes the rust stuff and returns a rust exception to Python.
    let matches = py_matcher.call(py, (py_path,), None)?.is_true(py)?;
    Ok(matches)
}

/// Extracts a Rust matcher from a Python Object
/// When possible it converts it into a pure-Rust matcher.
pub fn extract_matcher(py: Python, matcher: PyObject) -> PyResult<Arc<dyn Matcher + Sync + Send>> {
    if let Ok(matcher) = treematcher::downcast_from(py, matcher.clone_ref(py)) {
        return Ok(matcher.extract_inner(py));
    }
    if matcher.get_type(py).name(py).as_ref() == "treematcher" {
        return extract_matcher(py, matcher.getattr(py, "_matcher")?);
    }
    if matcher.get_type(py).name(py).as_ref() == "unionmatcher" {
        let py_matchers = matcher.getattr(py, "_matchers")?;
        let py_matchers = PyList::extract(py, &py_matchers)?;
        let mut matchers: Vec<Arc<dyn Matcher + Sync + Send>> = vec![];
        for matcher in py_matchers.iter(py) {
            matchers.push(extract_matcher(py, matcher)?);
        }

        return Ok(Arc::new(UnionMatcher::new(matchers)));
    }
    if matcher.get_type(py).name(py).as_ref() == "differencematcher" {
        let include = extract_matcher(py, matcher.getattr(py, "_m1")?)?;
        let exclude = extract_matcher(py, matcher.getattr(py, "_m2")?)?;
        return Ok(Arc::new(DifferenceMatcher::new(include, exclude)));
    }

    Ok(Arc::new(ThreadPythonMatcher::new(matcher)))
}

pub fn extract_option_matcher(
    py: Python,
    matcher: Option<PyObject>,
) -> PyResult<Arc<dyn Matcher + Sync + Send>> {
    match matcher {
        None => Ok(Arc::new(AlwaysMatcher::new())),
        Some(m) => extract_matcher(py, m),
    }
}
