/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#include "eden/fs/benchharness/Bench.h"
#include "eden/fs/utils/ImmediateFuture.h"

namespace {

using namespace facebook::eden;

void immediate_future(benchmark::State& state) {
  ImmediateFuture<uint64_t> fut{};

  for (auto _ : state) {
    auto newFut = std::move(fut).thenValue([](uint64_t v) { return v + 1; });
    fut = std::move(newFut);
  }
  state.SetItemsProcessed(std::move(fut).get());
}

void folly_future(benchmark::State& state) {
  folly::Future<int> fut{0};
  for (auto _ : state) {
    auto newFut = std::move(fut).thenValue([](int v) { return v + 1; });
    fut = std::move(newFut);
  }
  state.SetItemsProcessed(std::move(fut).get());
}

BENCHMARK(immediate_future);
BENCHMARK(folly_future);
} // namespace

EDEN_BENCHMARK_MAIN();
