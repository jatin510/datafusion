// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Values execution plan

use std::any::Any;
use std::sync::Arc;

use crate::execution_plan::{Boundedness, EmissionType};
use crate::memory::MemoryStream;
use crate::{common, DisplayAs, PlanProperties, SendableRecordBatchStream, Statistics};
use crate::{
    ColumnarValue, DisplayFormatType, ExecutionPlan, Partitioning, PhysicalExpr,
};
use arrow::datatypes::{Schema, SchemaRef};
use arrow::record_batch::{RecordBatch, RecordBatchOptions};
use datafusion_common::{internal_err, plan_err, Result, ScalarValue};
use datafusion_execution::TaskContext;
use datafusion_physical_expr::EquivalenceProperties;

/// Execution plan for values list based relation (produces constant rows)
#[deprecated(
    since = "45.0.0",
    note = "Use `MemorySourceConfig::try_new_as_values` instead"
)]
#[derive(Debug, Clone)]
pub struct ValuesExec {
    /// The schema
    schema: SchemaRef,
    /// The data
    data: Vec<RecordBatch>,
    /// Cache holding plan properties like equivalences, output partitioning etc.
    cache: PlanProperties,
}

#[allow(deprecated)]
impl ValuesExec {
    /// Create a new values exec from data as expr
    #[deprecated(since = "45.0.0", note = "Use `MemoryExec::try_new` instead")]
    pub fn try_new(
        schema: SchemaRef,
        data: Vec<Vec<Arc<dyn PhysicalExpr>>>,
    ) -> Result<Self> {
        if data.is_empty() {
            return plan_err!("Values list cannot be empty");
        }
        let n_row = data.len();
        let n_col = schema.fields().len();
        // We have this single row batch as a placeholder to satisfy evaluation argument
        // and generate a single output row
        let batch = RecordBatch::try_new_with_options(
            Arc::new(Schema::empty()),
            vec![],
            &RecordBatchOptions::new().with_row_count(Some(1)),
        )?;

        let arr = (0..n_col)
            .map(|j| {
                (0..n_row)
                    .map(|i| {
                        let r = data[i][j].evaluate(&batch);

                        match r {
                            Ok(ColumnarValue::Scalar(scalar)) => Ok(scalar),
                            Ok(ColumnarValue::Array(a)) if a.len() == 1 => {
                                ScalarValue::try_from_array(&a, 0)
                            }
                            Ok(ColumnarValue::Array(a)) => {
                                plan_err!(
                                    "Cannot have array values {a:?} in a values list"
                                )
                            }
                            Err(err) => Err(err),
                        }
                    })
                    .collect::<Result<Vec<_>>>()
                    .and_then(ScalarValue::iter_to_array)
            })
            .collect::<Result<Vec<_>>>()?;
        let batch = RecordBatch::try_new_with_options(
            Arc::clone(&schema),
            arr,
            &RecordBatchOptions::new().with_row_count(Some(n_row)),
        )?;
        let data: Vec<RecordBatch> = vec![batch];
        Self::try_new_from_batches(schema, data)
    }

    /// Create a new plan using the provided schema and batches.
    ///
    /// Errors if any of the batches don't match the provided schema, or if no
    /// batches are provided.
    #[deprecated(
        since = "45.0.0",
        note = "Use `MemoryExec::try_new_from_batches` instead"
    )]
    pub fn try_new_from_batches(
        schema: SchemaRef,
        batches: Vec<RecordBatch>,
    ) -> Result<Self> {
        if batches.is_empty() {
            return plan_err!("Values list cannot be empty");
        }

        for batch in &batches {
            let batch_schema = batch.schema();
            if batch_schema != schema {
                return plan_err!(
                    "Batch has invalid schema. Expected: {schema}, got: {batch_schema}"
                );
            }
        }

        let cache = Self::compute_properties(Arc::clone(&schema));
        #[allow(deprecated)]
        Ok(ValuesExec {
            schema,
            data: batches,
            cache,
        })
    }

    /// Provides the data
    pub fn data(&self) -> Vec<RecordBatch> {
        #[allow(deprecated)]
        self.data.clone()
    }

    /// This function creates the cache object that stores the plan properties such as schema, equivalence properties, ordering, partitioning, etc.
    fn compute_properties(schema: SchemaRef) -> PlanProperties {
        PlanProperties::new(
            EquivalenceProperties::new(schema),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        )
    }
}

#[allow(deprecated)]
impl DisplayAs for ValuesExec {
    fn fmt_as(
        &self,
        t: DisplayFormatType,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "ValuesExec")
            }
            DisplayFormatType::TreeRender => {
                // TODO: collect info
                write!(f, "")
            }
        }
    }
}

#[allow(deprecated)]
impl ExecutionPlan for ValuesExec {
    fn name(&self) -> &'static str {
        "ValuesExec"
    }

    /// Return a reference to Any that can be used for downcasting
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        #[allow(deprecated)]
        &self.cache
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        #[allow(deprecated)]
        ValuesExec::try_new_from_batches(Arc::clone(&self.schema), self.data.clone())
            .map(|e| Arc::new(e) as _)
    }

    fn execute(
        &self,
        partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        // ValuesExec has a single output partition
        if 0 != partition {
            return internal_err!(
                "ValuesExec invalid partition {partition} (expected 0)"
            );
        }

        Ok(Box::pin(MemoryStream::try_new(
            self.data(),
            #[allow(deprecated)]
            Arc::clone(&self.schema),
            None,
        )?))
    }

    fn statistics(&self) -> Result<Statistics> {
        let batch = self.data();
        Ok(common::compute_record_batch_statistics(
            &[batch],
            #[allow(deprecated)]
            &self.schema,
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expressions::lit;
    use crate::test::{self, make_partition};

    use arrow::datatypes::{DataType, Field};
    use datafusion_common::stats::{ColumnStatistics, Precision};

    #[tokio::test]
    async fn values_empty_case() -> Result<()> {
        let schema = test::aggr_test_schema();
        #[allow(deprecated)]
        let empty = ValuesExec::try_new(schema, vec![]);
        assert!(empty.is_err());
        Ok(())
    }

    #[test]
    fn new_exec_with_batches() {
        let batch = make_partition(7);
        let schema = batch.schema();
        let batches = vec![batch.clone(), batch];
        #[allow(deprecated)]
        let _exec = ValuesExec::try_new_from_batches(schema, batches).unwrap();
    }

    #[test]
    fn new_exec_with_batches_empty() {
        let batch = make_partition(7);
        let schema = batch.schema();
        #[allow(deprecated)]
        let _ = ValuesExec::try_new_from_batches(schema, Vec::new()).unwrap_err();
    }

    #[test]
    fn new_exec_with_batches_invalid_schema() {
        let batch = make_partition(7);
        let batches = vec![batch.clone(), batch];

        let invalid_schema = Arc::new(Schema::new(vec![
            Field::new("col0", DataType::UInt32, false),
            Field::new("col1", DataType::Utf8, false),
        ]));
        #[allow(deprecated)]
        let _ = ValuesExec::try_new_from_batches(invalid_schema, batches).unwrap_err();
    }

    // Test issue: https://github.com/apache/datafusion/issues/8763
    #[test]
    fn new_exec_with_non_nullable_schema() {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "col0",
            DataType::UInt32,
            false,
        )]));
        #[allow(deprecated)]
        let _ = ValuesExec::try_new(Arc::clone(&schema), vec![vec![lit(1u32)]]).unwrap();
        // Test that a null value is rejected
        #[allow(deprecated)]
        let _ = ValuesExec::try_new(schema, vec![vec![lit(ScalarValue::UInt32(None))]])
            .unwrap_err();
    }

    #[test]
    fn values_stats_with_nulls_only() -> Result<()> {
        let data = vec![
            vec![lit(ScalarValue::Null)],
            vec![lit(ScalarValue::Null)],
            vec![lit(ScalarValue::Null)],
        ];
        let rows = data.len();
        #[allow(deprecated)]
        let values = ValuesExec::try_new(
            Arc::new(Schema::new(vec![Field::new("col0", DataType::Null, true)])),
            data,
        )?;

        #[allow(deprecated)]
        let stats = values.statistics()?;
        assert_eq!(
            stats,
            Statistics {
                num_rows: Precision::Exact(rows),
                total_byte_size: Precision::Exact(8), // not important
                column_statistics: vec![ColumnStatistics {
                    null_count: Precision::Exact(rows), // there are only nulls
                    distinct_count: Precision::Absent,
                    max_value: Precision::Absent,
                    min_value: Precision::Absent,
                    sum_value: Precision::Absent,
                },],
            }
        );

        Ok(())
    }
}
