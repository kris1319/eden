// @generated SignedSource<<f5988f876f93477373b3ec069465cd91>>
// DO NOT EDIT THIS FILE MANUALLY!
// This file is a mechanical copy of the version in the configerator repo. To
// modify it, edit the copy in the configerator repo instead and copy it over by
// running the following in your fbcode directory:
//
// configerator-thrift-updater scm/mononoke/qps/qps_config.thrift

namespace rust mononoke.qps.config
namespace py3 mononoke.qps
namespace cpp2 mononoke.qps

// bumping SCS counters with category {category}
// and name {prefix}:{top_level_tier}:{src_region}:{dst_region}
struct QpsCountersConfig {
  1: string category,
  2: string prefix,
  3: string top_level_tier,
}


// Mononoke observability config
struct QpsConfig {
  1: QpsCountersConfig counters_config,
  2: list<QpsCountersConfig> counters_configs,
}

