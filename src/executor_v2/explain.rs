// Copyright 2023 RisingLight Project Authors. Licensed under Apache-2.0.

use futures::{future, stream};
use pretty_xmlish::PrettyConfig;

use super::*;
use crate::array::{ArrayImpl, Utf8Array};
use crate::planner::{costs, Explain};

/// The executor of `explain` statement.
pub struct ExplainExecutor {
    pub plan: RecExpr,
    pub catalog: RootCatalogRef,
}

impl ExplainExecutor {
    pub fn execute(self) -> BoxedExecutor {
        let binding = costs(&self.plan);
        let binding = Explain::of(&self.plan)
            .with_costs(&binding)
            .with_catalog(&self.catalog);
        let explainer = binding.pretty();
        let mut explain = String::with_capacity(4096);
        let mut config = PrettyConfig {
            need_boundaries: false,
            ..PrettyConfig::default()
        };
        config.unicode(&mut explain, &explainer);
        let chunk =
            DataChunk::from_iter([ArrayImpl::new_utf8(Utf8Array::from_iter([Some(explain)]))]);

        stream::once(future::ok(chunk)).boxed()
    }
}
