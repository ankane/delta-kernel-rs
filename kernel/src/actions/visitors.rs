//! This module defines visitors that can be used to extract the various delta actions from
//! [`crate::engine_data::EngineData`] types.

use std::collections::HashMap;

use crate::engine_data::{GetData, TypedGetData};
use crate::{DataVisitor, DeltaResult};

use super::Cdc;
use super::{
    deletion_vector::DeletionVectorDescriptor, Add, Format, Metadata, Protocol, Remove,
    SetTransaction,
};

#[derive(Default)]
#[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
#[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
struct MetadataVisitor {
    pub(crate) metadata: Option<Metadata>,
}

impl MetadataVisitor {
    #[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
    fn visit_metadata<'a>(
        row_index: usize,
        id: String,
        getters: &[&'a dyn GetData<'a>],
    ) -> DeltaResult<Metadata> {
        let name: Option<String> = getters[1].get_opt(row_index, "metadata.name")?;
        let description: Option<String> = getters[2].get_opt(row_index, "metadata.description")?;
        // get format out of primitives
        let format_provider: String = getters[3].get(row_index, "metadata.format.provider")?;
        // options for format is always empty, so skip getters[4]
        let schema_string: String = getters[5].get(row_index, "metadata.schema_string")?;
        let partition_columns: Vec<_> = getters[6].get(row_index, "metadata.partition_list")?;
        let created_time: Option<i64> = getters[7].get_opt(row_index, "metadata.created_time")?;
        let configuration_map_opt: Option<HashMap<_, _>> =
            getters[8].get_opt(row_index, "metadata.configuration")?;
        let configuration = configuration_map_opt.unwrap_or_else(HashMap::new);

        Ok(Metadata {
            id,
            name,
            description,
            format: Format {
                provider: format_provider,
                options: HashMap::new(),
            },
            schema_string,
            partition_columns,
            created_time,
            configuration,
        })
    }
}

impl DataVisitor for MetadataVisitor {
    fn visit<'a>(&mut self, row_count: usize, getters: &[&'a dyn GetData<'a>]) -> DeltaResult<()> {
        for i in 0..row_count {
            // Since id column is required, use it to detect presence of a metadata action
            if let Some(id) = getters[0].get_opt(i, "metadata.id")? {
                self.metadata = Some(Self::visit_metadata(i, id, getters)?);
                break;
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct SelectionVectorVisitor {
    pub(crate) selection_vector: Vec<bool>,
}

impl DataVisitor for SelectionVectorVisitor {
    fn visit<'a>(&mut self, row_count: usize, getters: &[&'a dyn GetData<'a>]) -> DeltaResult<()> {
        for i in 0..row_count {
            self.selection_vector
                .push(getters[0].get(i, "selectionvector.output")?)
        }
        Ok(())
    }
}

#[derive(Default)]
#[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
#[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
struct ProtocolVisitor {
    pub(crate) protocol: Option<Protocol>,
}

impl ProtocolVisitor {
    #[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
    fn visit_protocol<'a>(
        row_index: usize,
        min_reader_version: i32,
        getters: &[&'a dyn GetData<'a>],
    ) -> DeltaResult<Protocol> {
        let min_writer_version: i32 = getters[1].get(row_index, "protocol.min_writer_version")?;
        let reader_features: Option<Vec<_>> =
            getters[2].get_opt(row_index, "protocol.reader_features")?;
        let writer_features: Option<Vec<_>> =
            getters[3].get_opt(row_index, "protocol.writer_features")?;

        Protocol::try_new(
            min_reader_version,
            min_writer_version,
            reader_features,
            writer_features,
        )
    }
}

impl DataVisitor for ProtocolVisitor {
    fn visit<'a>(&mut self, row_count: usize, getters: &[&'a dyn GetData<'a>]) -> DeltaResult<()> {
        for i in 0..row_count {
            // Since minReaderVersion column is required, use it to detect presence of a Protocol action
            if let Some(mrv) = getters[0].get_opt(i, "protocol.min_reader_version")? {
                self.protocol = Some(Self::visit_protocol(i, mrv, getters)?);
                break;
            }
        }
        Ok(())
    }
}

#[derive(Default)]
#[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
#[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
struct AddVisitor {
    pub(crate) adds: Vec<Add>,
}

impl AddVisitor {
    #[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
    #[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
    fn visit_add<'a>(
        row_index: usize,
        path: String,
        getters: &[&'a dyn GetData<'a>],
    ) -> DeltaResult<Add> {
        let partition_values: HashMap<_, _> = getters[1].get(row_index, "add.partitionValues")?;
        let size: i64 = getters[2].get(row_index, "add.size")?;
        let modification_time: i64 = getters[3].get(row_index, "add.modificationTime")?;
        let data_change: bool = getters[4].get(row_index, "add.dataChange")?;
        let stats: Option<String> = getters[5].get_opt(row_index, "add.stats")?;

        // TODO(nick) extract tags if we ever need them at getters[6]

        let deletion_vector = visit_deletion_vector_at(row_index, &getters[7..])?;

        let base_row_id: Option<i64> = getters[12].get_opt(row_index, "add.base_row_id")?;
        let default_row_commit_version: Option<i64> =
            getters[13].get_opt(row_index, "add.default_row_commit")?;
        let clustering_provider: Option<String> =
            getters[14].get_opt(row_index, "add.clustering_provider")?;

        Ok(Add {
            path,
            partition_values,
            size,
            modification_time,
            data_change,
            stats,
            tags: None,
            deletion_vector,
            base_row_id,
            default_row_commit_version,
            clustering_provider,
        })
    }
}

impl DataVisitor for AddVisitor {
    fn visit<'a>(&mut self, row_count: usize, getters: &[&'a dyn GetData<'a>]) -> DeltaResult<()> {
        for i in 0..row_count {
            // Since path column is required, use it to detect presence of an Add action
            if let Some(path) = getters[0].get_opt(i, "add.path")? {
                self.adds.push(Self::visit_add(i, path, getters)?);
            }
        }
        Ok(())
    }
}

#[derive(Default)]
#[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
#[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
struct RemoveVisitor {
    pub(crate) removes: Vec<Remove>,
}

impl RemoveVisitor {
    #[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
    #[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
    fn visit_remove<'a>(
        row_index: usize,
        path: String,
        getters: &[&'a dyn GetData<'a>],
    ) -> DeltaResult<Remove> {
        let deletion_timestamp: Option<i64> =
            getters[1].get_opt(row_index, "remove.deletionTimestamp")?;
        let data_change: bool = getters[2].get(row_index, "remove.dataChange")?;
        let extended_file_metadata: Option<bool> =
            getters[3].get_opt(row_index, "remove.extendedFileMetadata")?;

        // TODO(nick) handle partition values in getters[4]

        let size: Option<i64> = getters[5].get_opt(row_index, "remove.size")?;

        // TODO(nick) tags are skipped in getters[6]

        let deletion_vector = visit_deletion_vector_at(row_index, &getters[7..])?;

        let base_row_id: Option<i64> = getters[12].get_opt(row_index, "remove.baseRowId")?;
        let default_row_commit_version: Option<i64> =
            getters[13].get_opt(row_index, "remove.defaultRowCommitVersion")?;

        Ok(Remove {
            path,
            data_change,
            deletion_timestamp,
            extended_file_metadata,
            partition_values: None,
            size,
            tags: None,
            deletion_vector,
            base_row_id,
            default_row_commit_version,
        })
    }
}

impl DataVisitor for RemoveVisitor {
    fn visit<'a>(&mut self, row_count: usize, getters: &[&'a dyn GetData<'a>]) -> DeltaResult<()> {
        for i in 0..row_count {
            // Since path column is required, use it to detect presence of an Remove action
            if let Some(path) = getters[0].get_opt(i, "remove.path")? {
                self.removes.push(Self::visit_remove(i, path, getters)?);
                break;
            }
        }
        Ok(())
    }
}

#[derive(Default)]
#[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
#[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
struct CdcVisitor {
    pub(crate) cdcs: Vec<Cdc>,
}

impl CdcVisitor {
    #[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
    #[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
    fn visit_cdc<'a>(
        row_index: usize,
        path: String,
        getters: &[&'a dyn GetData<'a>],
    ) -> DeltaResult<Cdc> {
        Ok(Cdc {
            path,
            partition_values: getters[1].get(row_index, "cdc.partitionValues")?,
            size: getters[2].get(row_index, "cdc.size")?,
            data_change: getters[3].get(row_index, "cdc.dataChange")?,
            tags: getters[4].get_opt(row_index, "cdc.tags")?,
        })
    }
}

impl DataVisitor for CdcVisitor {
    fn visit<'a>(&mut self, row_count: usize, getters: &[&'a dyn GetData<'a>]) -> DeltaResult<()> {
        for i in 0..row_count {
            // Since path column is required, use it to detect presence of an Add action
            if let Some(path) = getters[0].get_opt(i, "cdc.path")? {
                self.cdcs.push(Self::visit_cdc(i, path, getters)?);
            }
        }
        Ok(())
    }
}

pub type SetTransactionMap = HashMap<String, SetTransaction>;

/// Extact application transaction actions from the log into a map
///
/// This visitor maintains the first entry for each application id it
/// encounters.  When a specific application id is required then
/// `application_id` can be set. This bounds the memory required for the
/// visitor to at most one entry and reduces the amount of processing
/// required.
///
#[derive(Default, Debug)]
#[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
#[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
pub(crate) struct SetTransactionVisitor {
    pub(crate) set_transactions: SetTransactionMap,
    pub(crate) application_id: Option<String>,
}

impl SetTransactionVisitor {
    /// Create a new visitor. When application_id is set then bookkeeping is only for that id only
    pub(crate) fn new(application_id: Option<String>) -> Self {
        SetTransactionVisitor {
            set_transactions: HashMap::default(),
            application_id,
        }
    }

    #[cfg_attr(feature = "developer-visibility", visibility::make(pub))]
    #[cfg_attr(not(feature = "developer-visibility"), visibility::make(pub(crate)))]
    fn visit_txn<'a>(
        row_index: usize,
        app_id: String,
        getters: &[&'a dyn GetData<'a>],
    ) -> DeltaResult<SetTransaction> {
        let version: i64 = getters[1].get(row_index, "txn.version")?;
        let last_updated: Option<i64> = getters[2].get_long(row_index, "txn.lastUpdated")?;
        Ok(SetTransaction {
            app_id,
            version,
            last_updated,
        })
    }
}

impl DataVisitor for SetTransactionVisitor {
    fn visit<'a>(&mut self, row_count: usize, getters: &[&'a dyn GetData<'a>]) -> DeltaResult<()> {
        // Assumes batches are visited in reverse order relative to the log
        for i in 0..row_count {
            if let Some(app_id) = getters[0].get_opt(i, "txn.appId")? {
                // if caller requested a specific id then only visit matches
                if !self
                    .application_id
                    .as_ref()
                    .is_some_and(|requested| !requested.eq(&app_id))
                {
                    let txn = SetTransactionVisitor::visit_txn(i, app_id, getters)?;
                    if !self.set_transactions.contains_key(&txn.app_id) {
                        self.set_transactions.insert(txn.app_id.clone(), txn);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Get a DV out of some engine data. The caller is responsible for slicing the `getters` slice such
/// that the first element contains the `storageType` element of the deletion vector.
pub(crate) fn visit_deletion_vector_at<'a>(
    row_index: usize,
    getters: &[&'a dyn GetData<'a>],
) -> DeltaResult<Option<DeletionVectorDescriptor>> {
    if let Some(storage_type) =
        getters[0].get_opt(row_index, "remove.deletionVector.storageType")?
    {
        let path_or_inline_dv: String =
            getters[1].get(row_index, "deletionVector.pathOrInlineDv")?;
        let offset: Option<i32> = getters[2].get_opt(row_index, "deletionVector.offset")?;
        let size_in_bytes: i32 = getters[3].get(row_index, "deletionVector.sizeInBytes")?;
        let cardinality: i64 = getters[4].get(row_index, "deletionVector.cardinality")?;
        Ok(Some(DeletionVectorDescriptor {
            storage_type,
            path_or_inline_dv,
            offset,
            size_in_bytes,
            cardinality,
        }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema as ArrowSchema};

    use super::*;
    use crate::{
        actions::{get_log_add_schema, get_log_schema, CDC_NAME, SET_TRANSACTION_NAME},
        engine::{
            arrow_data::ArrowEngineData,
            sync::{json::SyncJsonHandler, SyncEngine},
        },
        Engine, EngineData, JsonHandler,
    };

    // TODO(nick): Merge all copies of this into one "test utils" thing
    fn string_array_to_engine_data(string_array: StringArray) -> Box<dyn EngineData> {
        let string_field = Arc::new(Field::new("a", DataType::Utf8, true));
        let schema = Arc::new(ArrowSchema::new(vec![string_field]));
        let batch = RecordBatch::try_new(schema, vec![Arc::new(string_array)])
            .expect("Can't convert to record batch");
        Box::new(ArrowEngineData::new(batch))
    }

    fn action_batch() -> Box<ArrowEngineData> {
        let handler = SyncJsonHandler {};
        let json_strings: StringArray = vec![
            r#"{"add":{"path":"part-00000-fae5310a-a37d-4e51-827b-c3d5516560ca-c000.snappy.parquet","partitionValues":{},"size":635,"modificationTime":1677811178336,"dataChange":true,"stats":"{\"numRecords\":10,\"minValues\":{\"value\":0},\"maxValues\":{\"value\":9},\"nullCount\":{\"value\":0},\"tightBounds\":true}","tags":{"INSERTION_TIME":"1677811178336000","MIN_INSERTION_TIME":"1677811178336000","MAX_INSERTION_TIME":"1677811178336000","OPTIMIZE_TARGET_SIZE":"268435456"}}}"#,
            r#"{"commitInfo":{"timestamp":1677811178585,"operation":"WRITE","operationParameters":{"mode":"ErrorIfExists","partitionBy":"[]"},"isolationLevel":"WriteSerializable","isBlindAppend":true,"operationMetrics":{"numFiles":"1","numOutputRows":"10","numOutputBytes":"635"},"engineInfo":"Databricks-Runtime/<unknown>","txnId":"a6a94671-55ef-450e-9546-b8465b9147de"}}"#,
            r#"{"protocol":{"minReaderVersion":3,"minWriterVersion":7,"readerFeatures":["deletionVectors"],"writerFeatures":["deletionVectors"]}}"#,
            r#"{"metaData":{"id":"testId","format":{"provider":"parquet","options":{}},"schemaString":"{\"type\":\"struct\",\"fields\":[{\"name\":\"value\",\"type\":\"integer\",\"nullable\":true,\"metadata\":{}}]}","partitionColumns":[],"configuration":{"delta.enableDeletionVectors":"true","delta.columnMapping.mode":"none", "delta.enableChangeDataFeed":"true"},"createdTime":1677811175819}}"#,
            r#"{"cdc":{"path":"_change_data/age=21/cdc-00000-93f7fceb-281a-446a-b221-07b88132d203.c000.snappy.parquet","partitionValues":{"age":"21"},"size":1033,"dataChange":false}}"#
        ]
        .into();
        let output_schema = get_log_schema().clone();
        let parsed = handler
            .parse_json(string_array_to_engine_data(json_strings), output_schema)
            .unwrap();
        ArrowEngineData::try_from_engine_data(parsed).unwrap()
    }

    #[test]
    fn test_parse_protocol() -> DeltaResult<()> {
        let data = action_batch();
        let parsed = Protocol::try_new_from_data(data.as_ref())?.unwrap();
        let expected = Protocol {
            min_reader_version: 3,
            min_writer_version: 7,
            reader_features: Some(vec!["deletionVectors".into()]),
            writer_features: Some(vec!["deletionVectors".into()]),
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn test_parse_cdc() -> DeltaResult<()> {
        let data = action_batch();
        let mut visitor = CdcVisitor::default();
        data.extract(get_log_schema().project(&[CDC_NAME])?, &mut visitor)?;
        let expected = Cdc {
            path: "_change_data/age=21/cdc-00000-93f7fceb-281a-446a-b221-07b88132d203.c000.snappy.parquet".into(),
            partition_values: HashMap::from([
                ("age".to_string(), "21".to_string()),
            ]),
            size: 1033,
            data_change: false,
            tags: None
        };

        assert_eq!(&visitor.cdcs, &[expected]);
        Ok(())
    }

    #[test]
    fn test_parse_metadata() -> DeltaResult<()> {
        let data = action_batch();
        let parsed = Metadata::try_new_from_data(data.as_ref())?.unwrap();

        let configuration = HashMap::from_iter([
            (
                "delta.enableDeletionVectors".to_string(),
                "true".to_string(),
            ),
            ("delta.columnMapping.mode".to_string(), "none".to_string()),
            ("delta.enableChangeDataFeed".to_string(), "true".to_string()),
        ]);
        let expected = Metadata {
            id: "testId".into(),
            name: None,
            description: None,
            format: Format {
                provider: "parquet".into(),
                options: Default::default(),
            },
            schema_string: r#"{"type":"struct","fields":[{"name":"value","type":"integer","nullable":true,"metadata":{}}]}"#.to_string(),
            partition_columns: Vec::new(),
            created_time: Some(1677811175819),
            configuration,
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn test_parse_add_partitioned() {
        let engine = SyncEngine::new();
        let json_handler = engine.get_json_handler();
        let json_strings: StringArray = vec![
            r#"{"commitInfo":{"timestamp":1670892998177,"operation":"WRITE","operationParameters":{"mode":"Append","partitionBy":"[\"c1\",\"c2\"]"},"isolationLevel":"Serializable","isBlindAppend":true,"operationMetrics":{"numFiles":"3","numOutputRows":"3","numOutputBytes":"1356"},"engineInfo":"Apache-Spark/3.3.1 Delta-Lake/2.2.0","txnId":"046a258f-45e3-4657-b0bf-abfb0f76681c"}}"#,
            r#"{"protocol":{"minReaderVersion":1,"minWriterVersion":2}}"#,
            r#"{"metaData":{"id":"aff5cb91-8cd9-4195-aef9-446908507302","format":{"provider":"parquet","options":{}},"schemaString":"{\"type\":\"struct\",\"fields\":[{\"name\":\"c1\",\"type\":\"integer\",\"nullable\":true,\"metadata\":{}},{\"name\":\"c2\",\"type\":\"string\",\"nullable\":true,\"metadata\":{}},{\"name\":\"c3\",\"type\":\"integer\",\"nullable\":true,\"metadata\":{}}]}","partitionColumns":["c1","c2"],"configuration":{},"createdTime":1670892997849}}"#,
            r#"{"add":{"path":"c1=4/c2=c/part-00003-f525f459-34f9-46f5-82d6-d42121d883fd.c000.snappy.parquet","partitionValues":{"c1":"4","c2":"c"},"size":452,"modificationTime":1670892998135,"dataChange":true,"stats":"{\"numRecords\":1,\"minValues\":{\"c3\":5},\"maxValues\":{\"c3\":5},\"nullCount\":{\"c3\":0}}"}}"#,
            r#"{"add":{"path":"c1=5/c2=b/part-00007-4e73fa3b-2c88-424a-8051-f8b54328ffdb.c000.snappy.parquet","partitionValues":{"c1":"5","c2":"b"},"size":452,"modificationTime":1670892998136,"dataChange":true,"stats":"{\"numRecords\":1,\"minValues\":{\"c3\":6},\"maxValues\":{\"c3\":6},\"nullCount\":{\"c3\":0}}"}}"#,
            r#"{"add":{"path":"c1=6/c2=a/part-00011-10619b10-b691-4fd0-acc4-2a9608499d7c.c000.snappy.parquet","partitionValues":{"c1":"6","c2":"a"},"size":452,"modificationTime":1670892998137,"dataChange":true,"stats":"{\"numRecords\":1,\"minValues\":{\"c3\":4},\"maxValues\":{\"c3\":4},\"nullCount\":{\"c3\":0}}"}}"#,
        ]
        .into();
        let output_schema = get_log_schema().clone();
        let batch = json_handler
            .parse_json(string_array_to_engine_data(json_strings), output_schema)
            .unwrap();
        let add_schema = get_log_add_schema().clone();
        let mut add_visitor = AddVisitor::default();
        batch.extract(add_schema, &mut add_visitor).unwrap();
        let add1 = Add {
            path: "c1=4/c2=c/part-00003-f525f459-34f9-46f5-82d6-d42121d883fd.c000.snappy.parquet".into(),
            partition_values: HashMap::from([
                ("c1".to_string(), "4".to_string()),
                ("c2".to_string(), "c".to_string()),
            ]),
            size: 452,
            modification_time: 1670892998135,
            data_change: true,
            stats: Some("{\"numRecords\":1,\"minValues\":{\"c3\":5},\"maxValues\":{\"c3\":5},\"nullCount\":{\"c3\":0}}".into()),
            tags: None,
            deletion_vector: None,
            base_row_id: None,
            default_row_commit_version: None,
            clustering_provider: None,
        };
        let add2 = Add {
            path: "c1=5/c2=b/part-00007-4e73fa3b-2c88-424a-8051-f8b54328ffdb.c000.snappy.parquet".into(),
            partition_values: HashMap::from([
                ("c1".to_string(), "5".to_string()),
                ("c2".to_string(), "b".to_string()),
            ]),
            modification_time: 1670892998136,
            stats: Some("{\"numRecords\":1,\"minValues\":{\"c3\":6},\"maxValues\":{\"c3\":6},\"nullCount\":{\"c3\":0}}".into()),
            ..add1.clone()
        };
        let add3 = Add {
            path: "c1=6/c2=a/part-00011-10619b10-b691-4fd0-acc4-2a9608499d7c.c000.snappy.parquet".into(),
            partition_values: HashMap::from([
                ("c1".to_string(), "6".to_string()),
                ("c2".to_string(), "a".to_string()),
            ]),
            modification_time: 1670892998137,
            stats: Some("{\"numRecords\":1,\"minValues\":{\"c3\":4},\"maxValues\":{\"c3\":4},\"nullCount\":{\"c3\":0}}".into()),
            ..add1.clone()
        };
        let expected = vec![add1, add2, add3];
        for (add, expected) in add_visitor.adds.into_iter().zip(expected.into_iter()) {
            assert_eq!(add, expected);
        }
    }

    #[test]
    fn test_parse_txn() {
        let engine = SyncEngine::new();
        let json_handler = engine.get_json_handler();
        let json_strings: StringArray = vec![
            r#"{"commitInfo":{"timestamp":1670892998177,"operation":"WRITE","operationParameters":{"mode":"Append","partitionBy":"[\"c1\",\"c2\"]"},"isolationLevel":"Serializable","isBlindAppend":true,"operationMetrics":{"numFiles":"3","numOutputRows":"3","numOutputBytes":"1356"},"engineInfo":"Apache-Spark/3.3.1 Delta-Lake/2.2.0","txnId":"046a258f-45e3-4657-b0bf-abfb0f76681c"}}"#,
            r#"{"protocol":{"minReaderVersion":1,"minWriterVersion":2}}"#,
            r#"{"metaData":{"id":"aff5cb91-8cd9-4195-aef9-446908507302","format":{"provider":"parquet","options":{}},"schemaString":"{\"type\":\"struct\",\"fields\":[{\"name\":\"c1\",\"type\":\"integer\",\"nullable\":true,\"metadata\":{}},{\"name\":\"c2\",\"type\":\"string\",\"nullable\":true,\"metadata\":{}},{\"name\":\"c3\",\"type\":\"integer\",\"nullable\":true,\"metadata\":{}}]}","partitionColumns":["c1","c2"],"configuration":{},"createdTime":1670892997849}}"#,
            r#"{"add":{"path":"c1=6/c2=a/part-00011-10619b10-b691-4fd0-acc4-2a9608499d7c.c000.snappy.parquet","partitionValues":{"c1":"6","c2":"a"},"size":452,"modificationTime":1670892998137,"dataChange":true,"stats":"{\"numRecords\":1,\"minValues\":{\"c3\":4},\"maxValues\":{\"c3\":4},\"nullCount\":{\"c3\":0}}"}}"#,
            r#"{"txn":{"appId":"myApp","version": 3}}"#,
            r#"{"txn":{"appId":"myApp2","version": 4, "lastUpdated": 1670892998177}}"#,
        ]
        .into();
        let output_schema = get_log_schema().clone();
        let batch = json_handler
            .parse_json(string_array_to_engine_data(json_strings), output_schema)
            .unwrap();
        let add_schema = get_log_schema()
            .project(&[SET_TRANSACTION_NAME])
            .expect("Can't get txn schema");
        let mut txn_visitor = SetTransactionVisitor::default();
        batch.extract(add_schema, &mut txn_visitor).unwrap();
        let mut actual = txn_visitor.set_transactions;
        assert_eq!(
            actual.remove("myApp2"),
            Some(SetTransaction {
                app_id: "myApp2".to_string(),
                version: 4,
                last_updated: Some(1670892998177),
            })
        );
        assert_eq!(
            actual.remove("myApp"),
            Some(SetTransaction {
                app_id: "myApp".to_string(),
                version: 3,
                last_updated: None,
            })
        );
    }
}
