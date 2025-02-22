/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#![allow(non_camel_case_types)]

use cpython::*;

use cats::{self, CatsSection};
use cpython_ext::{PyNone, ResultPyErrExt};
use pyconfigparser::config;

pub fn init_module(py: Python, package: &str) -> PyResult<PyModule> {
    let name = [package, "cats"].join(".");
    let m = PyModule::new(py, &name)?;

    m.add(
        py,
        "findcats",
        py_fn!(py, findcats(cfg: config, raise_if_missing: bool = true)),
    )?;

    m.add(
        py,
        "getcats",
        py_fn!(py, getcats(cfg: config, raise_if_missing: bool = true)),
    )?;

    Ok(m)
}

fn findcats(py: Python, cfg: config, raise_if_missing: bool) -> PyResult<PyObject> {
    let cfg = &cfg.get_cfg(py);

    CatsSection::from_config(cfg)
        .find_cats()
        .or_else(|e| if raise_if_missing { Err(e) } else { Ok(None) })
        .map_pyerr(py)?
        .map_or_else(
            || Ok(PyNone.to_py_object(py).into_object()),
            |group| {
                let dict = PyDict::new(py);

                if let Some(path) = group.path {
                    dict.set_item(py, "path", path.to_string_lossy())?;
                }

                if group.priority > 0 {
                    dict.set_item(py, "priority", group.priority)?;
                }

                Ok((&group.name, dict).to_py_object(py).into_object())
            },
        )
}

fn getcats(py: Python, cfg: config, raise_if_missing: bool) -> PyResult<PyObject> {
    let cfg = &cfg.get_cfg(py);

    CatsSection::from_config(cfg)
        .get_cats()
        .or_else(|e| if raise_if_missing { Err(e) } else { Ok(None) })
        .map_pyerr(py)?
        .map_or_else(
            || Ok(PyNone.to_py_object(py).into_object()),
            |cats_content| Ok(cats_content.to_py_object(py).into_object()),
        )
}
