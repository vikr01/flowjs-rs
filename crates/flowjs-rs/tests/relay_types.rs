//! Relay type compatibility tests.
//!
//! Tests whether flowjs-rs can accurately represent real-world Flow types from
//! Facebook's Relay codebase. Each test corresponds to a standalone .js file in
//! /tmp/flow-relay-test/ that was verified to pass `flow check`.
//!
//! For each test we:
//! 1. Define Rust types with `#[derive(Flow)]`
//! 2. Call `decl()` and compare with the expected Flow output
//! 3. Document which Flow constructs CANNOT be represented

#![allow(dead_code)]

use flowjs_rs::{Config, Flow};

fn cfg() -> Config {
    Config::new()
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 01: FeatureFlags — large struct with boolean fields
//
// Original Flow (simplified):
//   type FeatureFlags = {
//     ENABLE_VARIABLE_CONNECTION_KEY: boolean,
//     ENABLE_RELAY_RESOLVERS: boolean,
//     ...30 boolean fields...
//   };
//
// CANNOT represent: function-typed fields like
//   `BATCH_ASYNC_MODULE_UPDATES_FN: ?(() => void) => Disposable`
//   flowjs-rs can emit function types via fn pointers, but nullable function
//   fields where the function returns a custom type would need `#[flow(type = "...")]`.
//
// Fields use SCREAMING_SNAKE_CASE, which Rust fields also naturally use.
// BUT: flowjs-rs generates read-only (+) covariant fields by default,
//   while the original FeatureFlags uses mutable (no variance prefix) fields.
//   flowjs-rs CANNOT currently generate mutable object fields.
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
struct FeatureFlags {
    #[flow(rename = "ENABLE_VARIABLE_CONNECTION_KEY")]
    enable_variable_connection_key: bool,
    #[flow(rename = "ENABLE_RELAY_RESOLVERS")]
    enable_relay_resolvers: bool,
    #[flow(rename = "ENABLE_GETFRAGMENTIDENTIFIER_OPTIMIZATION")]
    enable_getfragmentidentifier_optimization: bool,
    #[flow(rename = "ENABLE_FRIENDLY_QUERY_NAME_GQL_URL")]
    enable_friendly_query_name_gql_url: bool,
    #[flow(rename = "ENABLE_DO_NOT_WRAP_LIVE_QUERY")]
    enable_do_not_wrap_live_query: bool,
    #[flow(rename = "ENABLE_NOTIFY_SUBSCRIPTION")]
    enable_notify_subscription: bool,
    // BATCH_ASYNC_MODULE_UPDATES_FN: skipped — nullable fn returning custom type
    #[flow(rename = "ENABLE_CONTAINERS_SUBSCRIBE_ON_COMMIT")]
    enable_containers_subscribe_on_commit: bool,
    #[flow(rename = "MAX_DATA_ID_LENGTH")]
    max_data_id_length: Option<i32>,
    #[flow(rename = "STRING_INTERN_LEVEL")]
    string_intern_level: i32,
    #[flow(rename = "LOG_MISSING_RECORDS_IN_PROD")]
    log_missing_records_in_prod: bool,
}

#[test]
fn file_01_feature_flags() {
    let c = cfg();
    let decl = FeatureFlags::decl(&c);
    assert!(decl.contains("type FeatureFlags"), "{decl}");
    assert!(
        decl.contains("ENABLE_VARIABLE_CONNECTION_KEY: boolean"),
        "{decl}"
    );
    assert!(
        decl.contains("ENABLE_RELAY_RESOLVERS: boolean"),
        "{decl}"
    );
    assert!(decl.contains("MAX_DATA_ID_LENGTH: ?number"), "{decl}");
    assert!(decl.contains("STRING_INTERN_LEVEL: number"), "{decl}");
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 02: ConnectionMetadata — struct with nullable union fields
//
// Original Flow:
//   type ConnectionMetadata = {
//     path: ?Array<string>,
//     direction: ?('forward' | 'backward' | 'bidirectional'),
//     cursor: ?string,
//     count: ?string,
//     stream?: boolean,
//     ...
//   };
//
// CANNOT represent:
// - Nullable string literal union `?('forward' | 'backward' | 'bidirectional')`
//   as a direct Flow type. We use `#[flow(type = "...")]` to emit the raw Flow.
// - Inexact object type (`...`) — flowjs-rs always generates exact objects `{||}`.
// - Mutable fields (no `+` prefix in the original).
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
struct ConnectionMetadata {
    path: Option<Vec<String>>,
    #[flow(type = "?('forward' | 'backward' | 'bidirectional')")]
    direction: Option<String>,
    cursor: Option<String>,
    count: Option<String>,
    #[flow(optional)]
    stream: Option<bool>,
}

#[test]
fn file_02_connection_metadata() {
    let c = cfg();
    let decl = ConnectionMetadata::decl(&c);
    assert!(decl.contains("type ConnectionMetadata"), "{decl}");
    assert!(
        decl.contains("path: ?$ReadOnlyArray<string>"),
        "{decl}"
    );
    assert!(
        decl.contains("direction: ?('forward' | 'backward' | 'bidirectional')"),
        "{decl}"
    );
    assert!(decl.contains("cursor: ?string"), "{decl}");
    assert!(decl.contains("count: ?string"), "{decl}");
    assert!(decl.contains("stream?: ?boolean"), "{decl}");
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 03: ReaderArgument types — tagged union with `kind` discriminator
//
// Original Flow:
//   type ReaderListValueArgument = {
//     +kind: 'ListValue',
//     +name: string,
//     +items: $ReadOnlyArray<ReaderArgument | null>,
//   };
//   ... (4 variants)
//   type ReaderArgument = Variant1 | Variant2 | Variant3 | Variant4;
//
// APPROACH: Use `#[flow(tag = "kind")]` internally tagged enum.
// Each variant maps to a different `kind` string literal.
//
// CANNOT represent:
// - `$ReadOnlyArray<ReaderArgument | null>` — Rust has no `Vec<Option<Self>>` with
//   null instead of undefined. We approximate with `Vec<Option<ReaderArgument>>`.
//   In Flow, `T | null` differs from `?T` (which is `T | null | void`).
//   flowjs-rs Option<T> → `?T`, not `T | null`.
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
#[flow(tag = "kind")]
enum ReaderArgument {
    #[flow(rename = "ListValue")]
    ListValue {
        name: String,
        #[flow(type = "$ReadOnlyArray<ReaderArgument | null>")]
        items: Vec<String>, // placeholder — type overridden
    },
    #[flow(rename = "Literal")]
    Literal {
        name: String,
        #[flow(rename = "type", optional)]
        type_field: Option<String>,
        #[flow(type = "mixed")]
        value: String, // placeholder — type overridden
    },
    #[flow(rename = "ObjectValue")]
    ObjectValue {
        name: String,
        #[flow(type = "$ReadOnlyArray<ReaderArgument>")]
        fields: Vec<String>, // placeholder — type overridden
    },
    #[flow(rename = "Variable")]
    Variable {
        name: String,
        #[flow(rename = "type", optional)]
        type_field: Option<String>,
        #[flow(rename = "variableName")]
        variable_name: String,
    },
}

#[test]
fn file_03_reader_arguments() {
    let c = cfg();
    let decl = ReaderArgument::decl(&c);
    assert!(decl.contains("type ReaderArgument"), "{decl}");
    // Check tag values
    assert!(decl.contains("kind: 'ListValue'"), "{decl}");
    assert!(decl.contains("kind: 'Literal'"), "{decl}");
    assert!(decl.contains("kind: 'ObjectValue'"), "{decl}");
    assert!(decl.contains("kind: 'Variable'"), "{decl}");
    // Check field names
    assert!(decl.contains("name: string"), "{decl}");
    assert!(
        decl.contains("items: $ReadOnlyArray<ReaderArgument | null>"),
        "{decl}"
    );
    assert!(decl.contains("value: mixed"), "{decl}");
    assert!(decl.contains("variableName: string"), "{decl}");
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 04: ReaderFields — tagged union of field types
//
// We model four object-with-`kind` types as an internally tagged enum.
//
// CANNOT represent:
// - `+plural: boolean` where it appears in some variants but not others (with
//   the internally tagged approach, we CAN model this; each variant has its own
//   fields).
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
#[flow(tag = "kind")]
enum ReaderField {
    #[flow(rename = "ScalarField")]
    ScalarField {
        #[flow(optional)]
        alias: Option<String>,
        name: String,
        #[flow(type = "?$ReadOnlyArray<mixed>", optional)]
        args: Option<Vec<String>>,
        #[flow(rename = "storageKey", optional)]
        storage_key: Option<String>,
    },
    #[flow(rename = "LinkedField")]
    LinkedField {
        #[flow(optional)]
        alias: Option<String>,
        name: String,
        #[flow(rename = "storageKey", optional)]
        storage_key: Option<String>,
        #[flow(type = "?$ReadOnlyArray<mixed>", optional)]
        args: Option<Vec<String>>,
        #[flow(rename = "concreteType", optional)]
        concrete_type: Option<String>,
        plural: bool,
        #[flow(type = "$ReadOnlyArray<mixed>")]
        selections: Vec<String>,
    },
    #[flow(rename = "RelayResolver")]
    RelayResolver {
        #[flow(optional)]
        alias: Option<String>,
        name: String,
        #[flow(type = "?$ReadOnlyArray<mixed>", optional)]
        args: Option<Vec<String>>,
        #[flow(type = "?mixed", optional)]
        fragment: Option<String>,
        path: String,
        #[flow(rename = "resolverModule", type = "mixed")]
        resolver_module: String,
        #[flow(rename = "normalizationInfo", optional)]
        normalization_info: Option<String>,
    },
    #[flow(rename = "RelayLiveResolver")]
    RelayLiveResolver {
        #[flow(optional)]
        alias: Option<String>,
        name: String,
        #[flow(type = "?$ReadOnlyArray<mixed>", optional)]
        args: Option<Vec<String>>,
        #[flow(type = "?mixed", optional)]
        fragment: Option<String>,
        path: String,
        #[flow(rename = "resolverModule", type = "mixed")]
        resolver_module: String,
        #[flow(rename = "normalizationInfo", optional)]
        normalization_info: Option<String>,
    },
}

#[test]
fn file_04_reader_fields() {
    let c = cfg();
    let decl = ReaderField::decl(&c);
    assert!(decl.contains("type ReaderField"), "{decl}");
    assert!(decl.contains("kind: 'ScalarField'"), "{decl}");
    assert!(decl.contains("kind: 'LinkedField'"), "{decl}");
    assert!(decl.contains("kind: 'RelayResolver'"), "{decl}");
    assert!(decl.contains("kind: 'RelayLiveResolver'"), "{decl}");
    assert!(decl.contains("plural: boolean"), "{decl}");
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 05: ReaderSelection — large type alias union
//
// Original Flow: `type ReaderSelection = A | B | C | ... | O;` (15 members)
//
// APPROACH: This is NOT a tagged union; it's a plain type alias that unions
//   separately-defined types. flowjs-rs cannot natively model "type X = A | B"
//   as a Rust enum (untagged enum newtype variants inline the inner type's
//   Flow representation, not its name). The correct approach is a container-
//   level `#[flow(type = "...")]` override that emits the raw Flow union.
//
// CANNOT represent natively:
// - Type alias to a union of separately-named types. There is no Rust enum
//   shape that would produce `type X = A | B | C;` — the untagged enum
//   always inlines each variant.
// - Workaround: use `#[flow(type = "...")]` container override.
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
#[flow(type = "ReaderCondition | ReaderClientEdge | ReaderClientExtension | ReaderDefer | ReaderField | ReaderActorChange | ReaderFragmentSpread | ReaderInlineDataFragmentSpread | ReaderAliasedInlineFragmentSpread | ReaderInlineFragment | ReaderModuleImport | ReaderStream | ReaderCatchField | ReaderRequiredField | ReaderRelayResolver")]
struct ReaderSelection;

#[test]
fn file_05_reader_selection() {
    let c = cfg();
    let decl = ReaderSelection::decl(&c);
    assert!(decl.contains("type ReaderSelection"), "{decl}");
    assert!(decl.contains("ReaderCondition"), "{decl}");
    assert!(decl.contains("ReaderClientEdge"), "{decl}");
    assert!(decl.contains("ReaderField"), "{decl}");
    assert!(decl.contains("ReaderRelayResolver"), "{decl}");
    // Should have 14 union separators for 15 types
    let pipes = decl.matches(" | ").count();
    assert_eq!(pipes, 14, "expected 14 union pipes, got {pipes}: {decl}");
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 06: RequestParameters — union of two object types
//
// Original Flow:
//   type RequestParameters =
//     | { +id: string, +text: string | null, ... }
//     | { +cacheID: string, +id: null, +text: string | null, ... };
//
// APPROACH: Untagged enum with two named-field variants.
//
// CANNOT represent:
// - `string | null` (distinct from `?string` which is `string | null | void`).
//   flowjs-rs Option<String> → `?string`, not `string | null`.
//   Use `#[flow(type = "string | null")]` override.
// - `'mutation' | 'query' | 'subscription'` string literal union.
//   Use `#[flow(type = "...")]` override.
// - Inexact objects (original uses `{[key: string]: unknown, ...}` for metadata).
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
#[flow(untagged)]
enum RequestParameters {
    WithId {
        id: String,
        #[flow(type = "string | null")]
        text: Option<String>,
        name: String,
        #[flow(rename = "operationKind", type = "'mutation' | 'query' | 'subscription'")]
        operation_kind: String,
    },
    WithCacheId {
        #[flow(rename = "cacheID")]
        cache_id: String,
        #[flow(type = "null")]
        id: Option<String>,
        #[flow(type = "string | null")]
        text: Option<String>,
        name: String,
        #[flow(rename = "operationKind", type = "'mutation' | 'query' | 'subscription'")]
        operation_kind: String,
    },
}

#[test]
fn file_06_request_parameters() {
    let c = cfg();
    let decl = RequestParameters::decl(&c);
    assert!(decl.contains("type RequestParameters"), "{decl}");
    assert!(decl.contains("id: string"), "{decl}");
    assert!(decl.contains("cacheID: string"), "{decl}");
    assert!(decl.contains("text: string | null"), "{decl}");
    assert!(
        decl.contains("operationKind: 'mutation' | 'query' | 'subscription'"),
        "{decl}"
    );
    assert!(decl.contains(" | "), "should be union: {decl}");
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 07: GraphQL response types — union of object types
//
// Tests: struct definitions + type alias union
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
struct GraphQLResponseWithData {
    #[flow(type = "PayloadData")]
    data: String,
    #[flow(type = "Array<PayloadError>", optional)]
    errors: Option<Vec<String>>,
    #[flow(type = "PayloadExtensions", optional)]
    extensions: Option<String>,
    #[flow(optional)]
    label: Option<String>,
    #[flow(type = "Array<string | number>", optional)]
    path: Option<Vec<String>>,
}

#[derive(Flow)]
struct GraphQLResponseWithoutData {
    #[flow(type = "?PayloadData", optional)]
    data: Option<String>,
    #[flow(type = "Array<PayloadError>")]
    errors: Vec<String>,
    #[flow(type = "PayloadExtensions", optional)]
    extensions: Option<String>,
    #[flow(optional)]
    label: Option<String>,
    #[flow(type = "Array<string | number>", optional)]
    path: Option<Vec<String>>,
}

#[derive(Flow)]
struct GraphQLResponseWithExtensionsOnly {
    #[flow(type = "null")]
    data: Option<String>,
    #[flow(type = "PayloadExtensions")]
    extensions: String,
}

#[derive(Flow)]
#[flow(untagged)]
enum GraphQLSingularResponse {
    WithData(GraphQLResponseWithData),
    WithExtensionsOnly(GraphQLResponseWithExtensionsOnly),
    WithoutData(GraphQLResponseWithoutData),
}

#[test]
fn file_07_graphql_response() {
    let c = cfg();
    // Individual structs
    let with_data = GraphQLResponseWithData::decl(&c);
    assert!(with_data.contains("data: PayloadData"), "{with_data}");
    assert!(
        with_data.contains("errors?: Array<PayloadError>"),
        "{with_data}"
    );

    let without_data = GraphQLResponseWithoutData::decl(&c);
    assert!(
        without_data.contains("data?: ?PayloadData"),
        "{without_data}"
    );
    assert!(
        without_data.contains("errors: Array<PayloadError>"),
        "{without_data}"
    );

    let ext_only = GraphQLResponseWithExtensionsOnly::decl(&c);
    assert!(ext_only.contains("data: null"), "{ext_only}");
    assert!(
        ext_only.contains("extensions: PayloadExtensions"),
        "{ext_only}"
    );

    // Union type — untagged enum inlines each variant's struct body
    let union_decl = GraphQLSingularResponse::decl(&c);
    assert!(
        union_decl.contains("type GraphQLSingularResponse"),
        "{union_decl}"
    );
    // The union has 3 branches separated by "|}" pattern (end of exact object)
    // Note: we cannot simply count " | " because type overrides like
    // "Array<string | number>" also contain " | ".
    let branch_count = union_decl.matches("|} | {|").count();
    assert_eq!(branch_count, 2, "expected 2 branch separators for 3 variants: {union_decl}");
    // Untagged newtype inlines the inner struct, so we see the full object
    // bodies, not type name references. Verify by checking fields from
    // each of the three structs appear in the union:
    // GraphQLResponseWithData has +data: PayloadData (required)
    // GraphQLResponseWithExtensionsOnly has +data: null
    // GraphQLResponseWithoutData has +errors: Array<PayloadError> (required)
    assert!(union_decl.contains("data: PayloadData"), "{union_decl}");
    assert!(union_decl.contains("data: null"), "{union_decl}");
    assert!(
        union_decl.contains("errors: Array<PayloadError>"),
        "{union_decl}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 08: DeclarativeMutationConfig — tagged union via `type` field
//
// Original Flow:
//   type DeclarativeMutationConfig = RangeAddConfig | RangeDeleteConfig | NodeDeleteConfig;
//   Each has a `type: 'RANGE_ADD' | 'RANGE_DELETE' | 'NODE_DELETE'` field.
//
// APPROACH: Internally tagged enum with `tag = "type"`.
//
// CANNOT represent:
// - Nested anonymous object types in arrays:
//   `connectionInfo?: Array<{ key: string, filters?: Variables, rangeBehavior: string }>`
//   Use `#[flow(type = "...")]` for these.
// - `string | Array<string>` union field.
// - Mutable fields (original has no `+` prefix).
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
#[flow(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
enum DeclarativeMutationConfig {
    RangeAdd {
        #[flow(rename = "parentName", optional)]
        parent_name: Option<String>,
        #[flow(rename = "parentID", optional)]
        parent_id: Option<String>,
        #[flow(rename = "edgeName")]
        edge_name: String,
        #[flow(rename = "connectionName", optional)]
        connection_name: Option<String>,
    },
    RangeDelete {
        #[flow(rename = "parentName", optional)]
        parent_name: Option<String>,
        #[flow(rename = "parentID", optional)]
        parent_id: Option<String>,
        #[flow(rename = "deletedIDFieldName", type = "string | Array<string>")]
        deleted_id_field_name: String,
        #[flow(rename = "pathToConnection", type = "Array<string>")]
        path_to_connection: Vec<String>,
        #[flow(rename = "connectionName", optional)]
        connection_name: Option<String>,
    },
    NodeDelete {
        #[flow(rename = "parentName", optional)]
        parent_name: Option<String>,
        #[flow(rename = "parentID", optional)]
        parent_id: Option<String>,
        #[flow(rename = "deletedIDFieldName")]
        deleted_id_field_name: String,
        #[flow(rename = "connectionName", optional)]
        connection_name: Option<String>,
    },
}

#[test]
fn file_08_mutation_config() {
    let c = cfg();
    let decl = DeclarativeMutationConfig::decl(&c);
    assert!(
        decl.contains("type DeclarativeMutationConfig"),
        "{decl}"
    );
    assert!(decl.contains("type: 'RANGE_ADD'"), "{decl}");
    assert!(decl.contains("type: 'RANGE_DELETE'"), "{decl}");
    assert!(decl.contains("type: 'NODE_DELETE'"), "{decl}");
    assert!(decl.contains("edgeName: string"), "{decl}");
    assert!(
        decl.contains("deletedIDFieldName: string | Array<string>"),
        "{decl}"
    );
    assert!(
        decl.contains("pathToConnection: Array<string>"),
        "{decl}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 09: NormalizationTypes — tagged union with self-referencing
//
// Subset of NormalizationNode types. Tests recursive types where
// NormalizationSelection references NormalizationCondition which references
// NormalizationSelection.
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
#[flow(tag = "kind")]
enum NormalizationArgument {
    #[flow(rename = "ListValue")]
    ListValue {
        name: String,
        #[flow(type = "$ReadOnlyArray<NormalizationArgument | null>")]
        items: Vec<String>,
    },
    #[flow(rename = "Literal")]
    Literal {
        name: String,
        #[flow(rename = "type", optional)]
        type_field: Option<String>,
        #[flow(type = "mixed")]
        value: String,
    },
    #[flow(rename = "ObjectValue")]
    ObjectValue {
        name: String,
        #[flow(type = "$ReadOnlyArray<NormalizationArgument>")]
        fields: Vec<String>,
    },
    #[flow(rename = "Variable")]
    Variable {
        name: String,
        #[flow(rename = "type", optional)]
        type_field: Option<String>,
        #[flow(rename = "variableName")]
        variable_name: String,
    },
}

#[derive(Flow)]
struct NormalizationScalarField {
    #[flow(type = "'ScalarField'")]
    kind: String,
    #[flow(optional)]
    alias: Option<String>,
    name: String,
    #[flow(type = "?$ReadOnlyArray<NormalizationArgument>", optional)]
    args: Option<Vec<String>>,
    #[flow(rename = "storageKey", optional)]
    storage_key: Option<String>,
}

#[derive(Flow)]
struct NormalizationLinkedField {
    #[flow(type = "'LinkedField'")]
    kind: String,
    #[flow(optional)]
    alias: Option<String>,
    name: String,
    #[flow(rename = "storageKey", optional)]
    storage_key: Option<String>,
    #[flow(type = "?$ReadOnlyArray<NormalizationArgument>", optional)]
    args: Option<Vec<String>>,
    #[flow(rename = "concreteType", optional)]
    concrete_type: Option<String>,
    plural: bool,
    #[flow(type = "$ReadOnlyArray<NormalizationSelection>")]
    selections: Vec<String>,
}

#[derive(Flow)]
struct NormalizationCondition {
    #[flow(type = "'Condition'")]
    kind: String,
    #[flow(rename = "passingValue")]
    passing_value: bool,
    condition: String,
    #[flow(type = "$ReadOnlyArray<NormalizationSelection>")]
    selections: Vec<String>,
}

// The union itself — must use container-level type override because
// flowjs-rs untagged enum newtype variants inline the inner type
// rather than referencing it by name.
#[derive(Flow)]
#[flow(type = "NormalizationCondition | NormalizationScalarField | NormalizationLinkedField | NormalizationInlineFragment | NormalizationDefer | NormalizationStream | NormalizationActorChange | NormalizationTypeDiscriminator | NormalizationClientExtension")]
struct NormalizationSelection;

#[test]
fn file_09_normalization_types() {
    let c = cfg();

    // NormalizationArgument — tagged union
    let arg_decl = NormalizationArgument::decl(&c);
    assert!(
        arg_decl.contains("type NormalizationArgument"),
        "{arg_decl}"
    );
    assert!(arg_decl.contains("kind: 'ListValue'"), "{arg_decl}");
    assert!(arg_decl.contains("kind: 'Variable'"), "{arg_decl}");

    // NormalizationScalarField — struct with literal kind field
    let scalar_decl = NormalizationScalarField::decl(&c);
    assert!(scalar_decl.contains("kind: 'ScalarField'"), "{scalar_decl}");
    assert!(scalar_decl.contains("name: string"), "{scalar_decl}");

    // NormalizationLinkedField — struct with selections array
    let linked_decl = NormalizationLinkedField::decl(&c);
    assert!(linked_decl.contains("kind: 'LinkedField'"), "{linked_decl}");
    assert!(linked_decl.contains("plural: boolean"), "{linked_decl}");
    assert!(
        linked_decl.contains("selections: $ReadOnlyArray<NormalizationSelection>"),
        "{linked_decl}"
    );

    // NormalizationSelection — large union (container-level type override)
    let sel_decl = NormalizationSelection::decl(&c);
    assert!(
        sel_decl.contains("type NormalizationSelection"),
        "{sel_decl}"
    );
    assert!(sel_decl.contains("NormalizationCondition"), "{sel_decl}");
    assert!(
        sel_decl.contains("NormalizationScalarField"),
        "{sel_decl}"
    );
    // 8 pipes for 9 members
    let sel_pipes = sel_decl.matches(" | ").count();
    assert_eq!(sel_pipes, 8, "expected 8 union pipes: {sel_decl}");
}

// ═══════════════════════════════════════════════════════════════════════════
// FILE 10: CacheConfig + utility types
//
// Tests: struct with optional function fields, type aliases for string
// literal unions, and newtype wrappers.
// ═══════════════════════════════════════════════════════════════════════════
#[derive(Flow)]
struct DataID(String);

#[derive(Flow)]
struct CacheConfig {
    #[flow(optional)]
    force: Option<bool>,
    #[flow(optional)]
    poll: Option<i32>,
    #[flow(rename = "liveConfigId", optional)]
    live_config_id: Option<String>,
    // Function fields: flowjs-rs can handle fn pointers
    #[flow(type = "() => void", optional)]
    on_subscribe: Option<fn() -> ()>,
    #[flow(rename = "transactionId", optional)]
    transaction_id: Option<String>,
}

// String literal unions as enums
#[derive(Flow)]
#[flow(flow_enum = "string", rename_all = "kebab-case")]
enum FetchQueryFetchPolicy {
    StoreOrNetwork,
    NetworkOnly,
}

#[derive(Flow)]
#[flow(flow_enum = "string")]
enum RenderPolicy {
    #[flow(rename = "full")]
    Full,
    #[flow(rename = "partial")]
    Partial,
}

#[derive(Flow)]
#[flow(flow_enum = "string")]
enum RequiredFieldAction {
    #[flow(rename = "NONE")]
    None,
    #[flow(rename = "LOG")]
    Log,
    #[flow(rename = "THROW")]
    Throw,
}

#[derive(Flow)]
#[flow(flow_enum = "string")]
enum CatchFieldTo {
    #[flow(rename = "RESULT")]
    Result,
    #[flow(rename = "NULL")]
    Null,
}

#[derive(Flow)]
struct RefetchableIdentifierInfo {
    #[flow(rename = "identifierField")]
    identifier_field: String,
    #[flow(rename = "identifierQueryVariableName")]
    identifier_query_variable_name: String,
}

#[derive(Flow)]
struct ReaderPaginationMetadata {
    #[flow(type = "{ +count: string, +cursor: string } | null")]
    backward: Option<String>,
    #[flow(type = "{ +count: string, +cursor: string } | null")]
    forward: Option<String>,
    #[flow(type = "$ReadOnlyArray<string>")]
    path: Vec<String>,
}

#[test]
fn file_10_cache_config_and_utils() {
    let c = cfg();

    // DataID — newtype
    let data_id_decl = DataID::decl(&c);
    assert_eq!(data_id_decl, "type DataID = string;");

    // CacheConfig — struct with optional fields
    let cache_decl = CacheConfig::decl(&c);
    assert!(cache_decl.contains("type CacheConfig"), "{cache_decl}");
    assert!(cache_decl.contains("force?: ?boolean"), "{cache_decl}");
    assert!(cache_decl.contains("poll?: ?number"), "{cache_decl}");
    assert!(
        cache_decl.contains("liveConfigId?: ?string"),
        "{cache_decl}"
    );
    // Function field with type override
    assert!(
        cache_decl.contains("on_subscribe?: () => void"),
        "{cache_decl}"
    );

    // FetchQueryFetchPolicy — Flow string enum with kebab-case
    let fetch_decl = FetchQueryFetchPolicy::decl(&c);
    assert!(
        fetch_decl.contains("enum FetchQueryFetchPolicy of string"),
        "{fetch_decl}"
    );
    assert!(
        fetch_decl.contains("StoreOrNetwork = 'store-or-network'"),
        "{fetch_decl}"
    );
    assert!(
        fetch_decl.contains("NetworkOnly = 'network-only'"),
        "{fetch_decl}"
    );

    // RenderPolicy — Flow string enum
    let render_decl = RenderPolicy::decl(&c);
    assert!(
        render_decl.contains("enum RenderPolicy of string"),
        "{render_decl}"
    );
    assert!(render_decl.contains("Full = 'full'"), "{render_decl}");
    assert!(
        render_decl.contains("Partial = 'partial'"),
        "{render_decl}"
    );

    // RequiredFieldAction — Flow string enum, SCREAMING values
    let req_decl = RequiredFieldAction::decl(&c);
    assert!(
        req_decl.contains("enum RequiredFieldAction of string"),
        "{req_decl}"
    );
    assert!(req_decl.contains("None = 'NONE'"), "{req_decl}");
    assert!(req_decl.contains("Log = 'LOG'"), "{req_decl}");
    assert!(req_decl.contains("Throw = 'THROW'"), "{req_decl}");

    // CatchFieldTo — Flow string enum
    let catch_decl = CatchFieldTo::decl(&c);
    assert!(
        catch_decl.contains("enum CatchFieldTo of string"),
        "{catch_decl}"
    );

    // RefetchableIdentifierInfo
    let refetch_decl = RefetchableIdentifierInfo::decl(&c);
    assert!(
        refetch_decl.contains("identifierField: string"),
        "{refetch_decl}"
    );
    assert!(
        refetch_decl.contains("identifierQueryVariableName: string"),
        "{refetch_decl}"
    );

    // ReaderPaginationMetadata
    let pagination_decl = ReaderPaginationMetadata::decl(&c);
    assert!(
        pagination_decl.contains("backward: { +count: string, +cursor: string } | null"),
        "{pagination_decl}"
    );
    assert!(
        pagination_decl.contains("path: $ReadOnlyArray<string>"),
        "{pagination_decl}"
    );
}
