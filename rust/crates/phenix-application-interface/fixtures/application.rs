// Generated from the fixed Phenix application descriptor. Do not edit.
pub const INTERFACE_ID: &str = "phenix.application@1";
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural0 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural1 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural2 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural3 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural4 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural5 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural6 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural7 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural8 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural9 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural10 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural11 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural12 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural13 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationError1Type { r#Cancelled,r#Closed,r#Conflict(Structural0),r#Disconnected,r#Failed(Structural1),r#InvalidInput(Structural2),r#InvalidPath(Structural3),r#InvalidResponse(Structural4),r#NotFound(Structural5),r#PermissionDenied(Structural6),r#SchemaMismatch(Structural7),r#StaleReference(Structural8),r#SubscriptionCapacity,r#TransactionConflict(Structural9),r#Unauthenticated(Structural10),r#UnknownValue(Structural11),r#UnsupportedCapability(Structural12),r#UnsupportedSnapshotPolicy(Structural13), }
impl phenix_core::PhenixContract for PhenixApplicationError1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.error@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeAcknowledged1Type {  }
impl phenix_core::PhenixContract for PhenixApplicationTypeAcknowledged1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.acknowledged@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeAuthenticateInput1Type { pub r#method_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeAuthenticateInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.authenticate-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeAuthenticationMethod1Type { pub r#description: Option<String>,pub r#id: String,pub r#name: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeAuthenticationMethod1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.authentication-method@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural14 { pub r#description: Option<String>,pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeAuthenticationMethods1Type { pub r#methods: Vec<Structural14>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeAuthenticationMethods1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.authentication-methods@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural15 { pub r#instructions: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeAuthenticationResult1Type { r#Authenticated,r#External(Structural15), }
impl phenix_core::PhenixContract for PhenixApplicationTypeAuthenticationResult1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.authentication-result@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeCallableInfo1Type { pub r#description: String,pub r#id: String,pub r#input: phenix_core::PhenixValue,pub r#output: phenix_core::PhenixValue, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCallableInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.callable-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeCallableInvokeInput1Type { pub r#callable_id: String,pub r#input: phenix_core::PhenixValue,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCallableInvokeInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.callable-invoke-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeCallableResult1Type { pub r#output: phenix_core::PhenixValue, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCallableResult1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.callable-result@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural16 { pub r#description: String,pub r#id: String,pub r#input: phenix_core::PhenixValue,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeCallables1Type { pub r#callables: Vec<Structural16>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCallables1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.callables@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeCapabilityList1Type { pub r#capabilities: Vec<String>,pub r#interface: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCapabilityList1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.capability-list@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeClientCallableRequest1Type { pub r#call_id: String,pub r#callable_id: String,pub r#execution_id: String,pub r#input: phenix_core::PhenixValue,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeClientCallableRequest1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.client-callable-request@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural17 { pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural20 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural21 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural22 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural23 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural24 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural25 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural26 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural27 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural28 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural29 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural30 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural31 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural32 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural33 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural19 { r#Cancelled,r#Closed,r#Conflict(Structural20),r#Disconnected,r#Failed(Structural21),r#InvalidInput(Structural22),r#InvalidPath(Structural23),r#InvalidResponse(Structural24),r#NotFound(Structural25),r#PermissionDenied(Structural26),r#SchemaMismatch(Structural27),r#StaleReference(Structural28),r#SubscriptionCapacity,r#TransactionConflict(Structural29),r#Unauthenticated(Structural30),r#UnknownValue(Structural31),r#UnsupportedCapability(Structural32),r#UnsupportedSnapshotPolicy(Structural33), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural18 { pub r#error: Structural19, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeClientCallableResponse1Type { r#Completed(Structural17),r#Failed(Structural18), }
impl phenix_core::PhenixContract for PhenixApplicationTypeClientCallableResponse1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.client-callable-response@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural34 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural35 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural36 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeContent1Type { r#Image(Structural34),r#Resource(Structural35),r#Text(Structural36), }
impl phenix_core::PhenixContract for PhenixApplicationTypeContent1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.content@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural37 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeDiagnostic1Type { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural37, }
impl phenix_core::PhenixContract for PhenixApplicationTypeDiagnostic1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.diagnostic@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural39 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural38 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural39, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeDiagnostics1Type { pub r#diagnostics: Vec<Structural38>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeDiagnostics1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.diagnostics@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeElicitationRequest1Type { pub r#message: String,pub r#schema: phenix_core::PhenixValue,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeElicitationRequest1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.elicitation-request@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural40 { pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeElicitationResponse1Type { r#Accepted(Structural40),r#Cancelled,r#Declined, }
impl phenix_core::PhenixContract for PhenixApplicationTypeElicitationResponse1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.elicitation-response@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeEmpty1Type {  }
impl phenix_core::PhenixContract for PhenixApplicationTypeEmpty1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.empty@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural41 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural46 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural47 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural48 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural49 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural50 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural51 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural52 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural53 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural54 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural55 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural56 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural57 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural58 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural59 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural45 { r#Cancelled,r#Closed,r#Conflict(Structural46),r#Disconnected,r#Failed(Structural47),r#InvalidInput(Structural48),r#InvalidPath(Structural49),r#InvalidResponse(Structural50),r#NotFound(Structural51),r#PermissionDenied(Structural52),r#SchemaMismatch(Structural53),r#StaleReference(Structural54),r#SubscriptionCapacity,r#TransactionConflict(Structural55),r#Unauthenticated(Structural56),r#UnknownValue(Structural57),r#UnsupportedCapability(Structural58),r#UnsupportedSnapshotPolicy(Structural59), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural44 { pub r#error: Structural45, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural43 { r#Cancelled,r#Completed,r#Failed(Structural44),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural42 { pub r#state: Structural43, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural60 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural63 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural64 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural65 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural66 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural67 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural68 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural69 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural70 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural71 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural72 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural73 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural74 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural75 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural76 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural62 { r#Cancelled,r#Closed,r#Conflict(Structural63),r#Disconnected,r#Failed(Structural64),r#InvalidInput(Structural65),r#InvalidPath(Structural66),r#InvalidResponse(Structural67),r#NotFound(Structural68),r#PermissionDenied(Structural69),r#SchemaMismatch(Structural70),r#StaleReference(Structural71),r#SubscriptionCapacity,r#TransactionConflict(Structural72),r#Unauthenticated(Structural73),r#UnknownValue(Structural74),r#UnsupportedCapability(Structural75),r#UnsupportedSnapshotPolicy(Structural76), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural61 { pub r#call_id: String,pub r#error: Structural62, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural77 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeExecutionChange1Type { r#Progress(Structural41),r#State(Structural42),r#ToolCall(Structural60),r#ToolFailed(Structural61),r#ToolResult(Structural77), }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionChange1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-change@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural81 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural82 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural83 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural84 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural85 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural86 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural87 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural88 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural89 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural90 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural91 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural92 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural93 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural94 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural80 { r#Cancelled,r#Closed,r#Conflict(Structural81),r#Disconnected,r#Failed(Structural82),r#InvalidInput(Structural83),r#InvalidPath(Structural84),r#InvalidResponse(Structural85),r#NotFound(Structural86),r#PermissionDenied(Structural87),r#SchemaMismatch(Structural88),r#StaleReference(Structural89),r#SubscriptionCapacity,r#TransactionConflict(Structural90),r#Unauthenticated(Structural91),r#UnknownValue(Structural92),r#UnsupportedCapability(Structural93),r#UnsupportedSnapshotPolicy(Structural94), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural79 { pub r#error: Structural80, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural78 { r#Cancelled,r#Completed,r#Failed(Structural79),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionInfo1Type { pub r#execution_id: String,pub r#parent: Option<String>,pub r#state: Structural78, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionInput1Type { pub r#execution_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural97 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural98 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural99 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural100 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural101 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural102 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural103 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural104 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural105 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural106 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural107 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural108 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural109 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural110 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural96 { r#Cancelled,r#Closed,r#Conflict(Structural97),r#Disconnected,r#Failed(Structural98),r#InvalidInput(Structural99),r#InvalidPath(Structural100),r#InvalidResponse(Structural101),r#NotFound(Structural102),r#PermissionDenied(Structural103),r#SchemaMismatch(Structural104),r#StaleReference(Structural105),r#SubscriptionCapacity,r#TransactionConflict(Structural106),r#Unauthenticated(Structural107),r#UnknownValue(Structural108),r#UnsupportedCapability(Structural109),r#UnsupportedSnapshotPolicy(Structural110), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural95 { pub r#error: Structural96, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeExecutionState1Type { r#Cancelled,r#Completed,r#Failed(Structural95),r#Pending,r#Running, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionState1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-state@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural115 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural116 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural117 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural118 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural119 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural120 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural121 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural122 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural123 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural124 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural125 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural126 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural127 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural128 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural114 { r#Cancelled,r#Closed,r#Conflict(Structural115),r#Disconnected,r#Failed(Structural116),r#InvalidInput(Structural117),r#InvalidPath(Structural118),r#InvalidResponse(Structural119),r#NotFound(Structural120),r#PermissionDenied(Structural121),r#SchemaMismatch(Structural122),r#StaleReference(Structural123),r#SubscriptionCapacity,r#TransactionConflict(Structural124),r#Unauthenticated(Structural125),r#UnknownValue(Structural126),r#UnsupportedCapability(Structural127),r#UnsupportedSnapshotPolicy(Structural128), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural113 { pub r#error: Structural114, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural112 { r#Cancelled,r#Completed,r#Failed(Structural113),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural111 { pub r#execution_id: String,pub r#parent: Option<String>,pub r#state: Structural112, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionTree1Type { pub r#executions: Vec<Structural111>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionTree1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-tree@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural130 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural135 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural136 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural137 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural138 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural139 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural140 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural141 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural142 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural143 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural144 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural145 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural146 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural147 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural148 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural134 { r#Cancelled,r#Closed,r#Conflict(Structural135),r#Disconnected,r#Failed(Structural136),r#InvalidInput(Structural137),r#InvalidPath(Structural138),r#InvalidResponse(Structural139),r#NotFound(Structural140),r#PermissionDenied(Structural141),r#SchemaMismatch(Structural142),r#StaleReference(Structural143),r#SubscriptionCapacity,r#TransactionConflict(Structural144),r#Unauthenticated(Structural145),r#UnknownValue(Structural146),r#UnsupportedCapability(Structural147),r#UnsupportedSnapshotPolicy(Structural148), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural133 { pub r#error: Structural134, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural132 { r#Cancelled,r#Completed,r#Failed(Structural133),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural131 { pub r#state: Structural132, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural149 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural152 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural153 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural154 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural155 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural156 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural157 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural158 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural159 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural160 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural161 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural162 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural163 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural164 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural165 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural151 { r#Cancelled,r#Closed,r#Conflict(Structural152),r#Disconnected,r#Failed(Structural153),r#InvalidInput(Structural154),r#InvalidPath(Structural155),r#InvalidResponse(Structural156),r#NotFound(Structural157),r#PermissionDenied(Structural158),r#SchemaMismatch(Structural159),r#StaleReference(Structural160),r#SubscriptionCapacity,r#TransactionConflict(Structural161),r#Unauthenticated(Structural162),r#UnknownValue(Structural163),r#UnsupportedCapability(Structural164),r#UnsupportedSnapshotPolicy(Structural165), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural150 { pub r#call_id: String,pub r#error: Structural151, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural166 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural129 { r#Progress(Structural130),r#State(Structural131),r#ToolCall(Structural149),r#ToolFailed(Structural150),r#ToolResult(Structural166), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionUpdate1Type { pub r#execution_id: String,pub r#sequence: u64,pub r#session_id: String,pub r#update: Structural129, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionUpdate1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-update@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeMessageRole1Type { r#Assistant,r#User, }
impl phenix_core::PhenixContract for PhenixApplicationTypeMessageRole1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.message-role@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural168 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural169 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural170 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural167 { r#Image(Structural168),r#Resource(Structural169),r#Text(Structural170), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural171 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeMessage1Type { pub r#content: Vec<Structural167>,pub r#role: Structural171, }
impl phenix_core::PhenixContract for PhenixApplicationTypeMessage1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.message@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeModelInfo1Type { pub r#description: Option<String>,pub r#id: String,pub r#name: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeModelInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.model-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeModelSelectInput1Type { pub r#model_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeModelSelectInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.model-select-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural172 { pub r#description: Option<String>,pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeModels1Type { pub r#available: Vec<Structural172>,pub r#selected: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeModels1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.models@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural175 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural176 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural177 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural174 { r#Field(Structural175),r#Index(Structural176),r#MapKey(Structural177),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural173 { pub r#segments: Vec<Structural174>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableAddress1Type { pub r#path: Structural173,pub r#value_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableAddress1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-address@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural182 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural183 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural184 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural181 { r#Field(Structural182),r#Index(Structural183),r#MapKey(Structural184),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural180 { pub r#segments: Vec<Structural181>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural179 { pub r#path: Structural180, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural178 { pub r#change: Structural179, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural189 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural190 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural191 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural188 { r#Field(Structural189),r#Index(Structural190),r#MapKey(Structural191),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural187 { pub r#segments: Vec<Structural188>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural186 { pub r#path: Structural187,pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural185 { pub r#change: Structural186, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural196 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural197 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural198 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural195 { r#Field(Structural196),r#Index(Structural197),r#MapKey(Structural198),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural194 { pub r#segments: Vec<Structural195>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural193 { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural194,pub r#start: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural192 { pub r#change: Structural193, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableChange1Type { r#Remove(Structural178),r#Replace(Structural185),r#Splice(Structural192), }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableChange1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-change@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural202 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural203 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural204 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural201 { r#Field(Structural202),r#Index(Structural203),r#MapKey(Structural204),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural200 { pub r#segments: Vec<Structural201>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural199 { pub r#path: Structural200,pub r#value_id: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural212 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural213 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural214 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural211 { r#Field(Structural212),r#Index(Structural213),r#MapKey(Structural214),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural210 { pub r#segments: Vec<Structural211>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural209 { pub r#path: Structural210, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural208 { pub r#change: Structural209, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural219 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural220 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural221 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural218 { r#Field(Structural219),r#Index(Structural220),r#MapKey(Structural221),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural217 { pub r#segments: Vec<Structural218>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural216 { pub r#path: Structural217,pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural215 { pub r#change: Structural216, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural226 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural227 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural228 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural225 { r#Field(Structural226),r#Index(Structural227),r#MapKey(Structural228),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural224 { pub r#segments: Vec<Structural225>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural223 { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural224,pub r#start: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural222 { pub r#change: Structural223, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural207 { r#Remove(Structural208),r#Replace(Structural215),r#Splice(Structural222), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural206 { pub r#changes: Vec<Structural207>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural229 { pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural205 { r#Diff(Structural206),r#Full(Structural229), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableDelivery1Type { pub r#address: Structural199,pub r#commit_id: Option<u64>,pub r#from_version: u64,pub r#generation: u64,pub r#payload: Structural205,pub r#subscription_id: u64,pub r#value_id: String,pub r#version: u64, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableDelivery1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-delivery@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural232 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural233 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural234 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural231 { r#Field(Structural232),r#Index(Structural233),r#MapKey(Structural234),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural230 { pub r#segments: Vec<Structural231>, }
#[derive(Clone, Debug, PartialEq)]
pub struct Object235(pub phenix_core::ObjectRef);
impl phenix_core::ValueCodec for Object235 { fn phenix_type() -> phenix_core::PhenixSchema { phenix_core::PhenixSchema::Object { contract: phenix_core::ContractId::parse("phenix.observable@1").expect("generated object contract is valid") } } fn to_value(&self) -> phenix_core::PhenixValue { phenix_core::PhenixValue::Object(self.0.clone()) } fn from_value(value: &phenix_core::PhenixValue) -> Result<Self, phenix_core::ValueError> { <Self as phenix_core::ValueCodec>::phenix_type().parse(value)?; match value { phenix_core::PhenixValue::Object(reference) => Ok(Self(reference.clone())), _ => unreachable!("validated object value"), } } }
impl From<&Object235> for phenix_core::PhenixValue { fn from(value: &Object235) -> Self { <Object235 as phenix_core::ValueCodec>::to_value(value) } }
impl TryFrom<phenix_core::Exact<&phenix_core::PhenixValue>> for Object235 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Exact<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::from_value(value.0) } }
impl TryFrom<phenix_core::Project<&phenix_core::PhenixValue>> for Object235 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Project<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::project_from_value(value.0) } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableGetInput1Type { pub r#path: Structural230,pub r#reference: Object235, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableGetInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-get-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableInitial1Type { r#Full,r#None, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableInitial1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-initial@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural239 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural240 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural241 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural238 { r#Field(Structural239),r#Index(Structural240),r#MapKey(Structural241),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural237 { pub r#segments: Vec<Structural238>, }
#[derive(Clone, Debug, PartialEq)]
pub struct Object242(pub phenix_core::ObjectRef);
impl phenix_core::ValueCodec for Object242 { fn phenix_type() -> phenix_core::PhenixSchema { phenix_core::PhenixSchema::Object { contract: phenix_core::ContractId::parse("phenix.observable@1").expect("generated object contract is valid") } } fn to_value(&self) -> phenix_core::PhenixValue { phenix_core::PhenixValue::Object(self.0.clone()) } fn from_value(value: &phenix_core::PhenixValue) -> Result<Self, phenix_core::ValueError> { <Self as phenix_core::ValueCodec>::phenix_type().parse(value)?; match value { phenix_core::PhenixValue::Object(reference) => Ok(Self(reference.clone())), _ => unreachable!("validated object value"), } } }
impl From<&Object242> for phenix_core::PhenixValue { fn from(value: &Object242) -> Self { <Object242 as phenix_core::ValueCodec>::to_value(value) } }
impl TryFrom<phenix_core::Exact<&phenix_core::PhenixValue>> for Object242 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Exact<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::from_value(value.0) } }
impl TryFrom<phenix_core::Project<&phenix_core::PhenixValue>> for Object242 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Project<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::project_from_value(value.0) } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural243 { r#CopyOnChange,r#CurrentOnly, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural236 { pub r#binding_path: Vec<String>,pub r#namespace: String,pub r#path: Structural237,pub r#reference: Object242,pub r#resource: String,pub r#schema: phenix_core::PhenixValue,pub r#snapshot_policy: Structural243,pub r#value_id: String,pub r#version: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableList1Type { pub r#resources: Vec<Structural236>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableList1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-list@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableMode1Type { r#Diff,r#Full, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableMode1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-mode@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural244 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural245 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural246 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservablePathSegment1Type { r#Field(Structural244),r#Index(Structural245),r#MapKey(Structural246),r#OptionPayload,r#VariantPayload, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservablePathSegment1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-path-segment@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural248 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural249 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural250 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural247 { r#Field(Structural248),r#Index(Structural249),r#MapKey(Structural250),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservablePath1Type { pub r#segments: Vec<Structural247>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservablePath1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-path@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural257 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural258 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural259 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural256 { r#Field(Structural257),r#Index(Structural258),r#MapKey(Structural259),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural255 { pub r#segments: Vec<Structural256>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural254 { pub r#path: Structural255, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural253 { pub r#change: Structural254, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural264 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural265 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural266 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural263 { r#Field(Structural264),r#Index(Structural265),r#MapKey(Structural266),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural262 { pub r#segments: Vec<Structural263>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural261 { pub r#path: Structural262,pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural260 { pub r#change: Structural261, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural271 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural272 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural273 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural270 { r#Field(Structural271),r#Index(Structural272),r#MapKey(Structural273),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural269 { pub r#segments: Vec<Structural270>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural268 { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural269,pub r#start: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural267 { pub r#change: Structural268, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural252 { r#Remove(Structural253),r#Replace(Structural260),r#Splice(Structural267), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural251 { pub r#changes: Vec<Structural252>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural274 { pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservablePayload1Type { r#Diff(Structural251),r#Full(Structural274), }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservablePayload1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-payload@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural277 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural278 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural279 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural276 { r#Field(Structural277),r#Index(Structural278),r#MapKey(Structural279),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural275 { pub r#segments: Vec<Structural276>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableRemove1Type { pub r#path: Structural275, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableRemove1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-remove@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural282 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural283 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural284 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural281 { r#Field(Structural282),r#Index(Structural283),r#MapKey(Structural284),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural280 { pub r#segments: Vec<Structural281>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableReplace1Type { pub r#path: Structural280,pub r#value: phenix_core::PhenixValue, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableReplace1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-replace@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural287 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural288 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural289 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural286 { r#Field(Structural287),r#Index(Structural288),r#MapKey(Structural289),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural285 { pub r#segments: Vec<Structural286>, }
#[derive(Clone, Debug, PartialEq)]
pub struct Object290(pub phenix_core::ObjectRef);
impl phenix_core::ValueCodec for Object290 { fn phenix_type() -> phenix_core::PhenixSchema { phenix_core::PhenixSchema::Object { contract: phenix_core::ContractId::parse("phenix.observable@1").expect("generated object contract is valid") } } fn to_value(&self) -> phenix_core::PhenixValue { phenix_core::PhenixValue::Object(self.0.clone()) } fn from_value(value: &phenix_core::PhenixValue) -> Result<Self, phenix_core::ValueError> { <Self as phenix_core::ValueCodec>::phenix_type().parse(value)?; match value { phenix_core::PhenixValue::Object(reference) => Ok(Self(reference.clone())), _ => unreachable!("validated object value"), } } }
impl From<&Object290> for phenix_core::PhenixValue { fn from(value: &Object290) -> Self { <Object290 as phenix_core::ValueCodec>::to_value(value) } }
impl TryFrom<phenix_core::Exact<&phenix_core::PhenixValue>> for Object290 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Exact<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::from_value(value.0) } }
impl TryFrom<phenix_core::Project<&phenix_core::PhenixValue>> for Object290 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Project<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::project_from_value(value.0) } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural291 { r#CopyOnChange,r#CurrentOnly, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableResource1Type { pub r#binding_path: Vec<String>,pub r#namespace: String,pub r#path: Structural285,pub r#reference: Object290,pub r#resource: String,pub r#schema: phenix_core::PhenixValue,pub r#snapshot_policy: Structural291,pub r#value_id: String,pub r#version: u64, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableResource1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-resource@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableScope1Type { r#Exact,r#Recursive, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableScope1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-scope@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableSnapshotPolicy1Type { r#CopyOnChange,r#CurrentOnly, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableSnapshotPolicy1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-snapshot-policy@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural294 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural295 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural296 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural293 { r#Field(Structural294),r#Index(Structural295),r#MapKey(Structural296),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural292 { pub r#segments: Vec<Structural293>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableSplice1Type { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural292,pub r#start: u64, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableSplice1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-splice@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural297 { r#Full,r#None, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural298 { r#Diff,r#Full, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural301 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural302 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural303 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural300 { r#Field(Structural301),r#Index(Structural302),r#MapKey(Structural303),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural299 { pub r#segments: Vec<Structural300>, }
#[derive(Clone, Debug, PartialEq)]
pub struct Object304(pub phenix_core::ObjectRef);
impl phenix_core::ValueCodec for Object304 { fn phenix_type() -> phenix_core::PhenixSchema { phenix_core::PhenixSchema::Object { contract: phenix_core::ContractId::parse("phenix.observable@1").expect("generated object contract is valid") } } fn to_value(&self) -> phenix_core::PhenixValue { phenix_core::PhenixValue::Object(self.0.clone()) } fn from_value(value: &phenix_core::PhenixValue) -> Result<Self, phenix_core::ValueError> { <Self as phenix_core::ValueCodec>::phenix_type().parse(value)?; match value { phenix_core::PhenixValue::Object(reference) => Ok(Self(reference.clone())), _ => unreachable!("validated object value"), } } }
impl From<&Object304> for phenix_core::PhenixValue { fn from(value: &Object304) -> Self { <Object304 as phenix_core::ValueCodec>::to_value(value) } }
impl TryFrom<phenix_core::Exact<&phenix_core::PhenixValue>> for Object304 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Exact<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::from_value(value.0) } }
impl TryFrom<phenix_core::Project<&phenix_core::PhenixValue>> for Object304 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Project<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::project_from_value(value.0) } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural305 { r#Exact,r#Recursive, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableSubscribeInput1Type { pub r#initial: Structural297,pub r#mode: Structural298,pub r#path: Structural299,pub r#reference: Object304,pub r#scope: Structural305, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableSubscribeInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-subscribe-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural310 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural311 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural312 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural309 { r#Field(Structural310),r#Index(Structural311),r#MapKey(Structural312),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural308 { pub r#segments: Vec<Structural309>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural307 { pub r#path: Structural308,pub r#value_id: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural320 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural321 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural322 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural319 { r#Field(Structural320),r#Index(Structural321),r#MapKey(Structural322),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural318 { pub r#segments: Vec<Structural319>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural317 { pub r#path: Structural318, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural316 { pub r#change: Structural317, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural327 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural328 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural329 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural326 { r#Field(Structural327),r#Index(Structural328),r#MapKey(Structural329),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural325 { pub r#segments: Vec<Structural326>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural324 { pub r#path: Structural325,pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural323 { pub r#change: Structural324, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural334 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural335 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural336 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural333 { r#Field(Structural334),r#Index(Structural335),r#MapKey(Structural336),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural332 { pub r#segments: Vec<Structural333>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural331 { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural332,pub r#start: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural330 { pub r#change: Structural331, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural315 { r#Remove(Structural316),r#Replace(Structural323),r#Splice(Structural330), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural314 { pub r#changes: Vec<Structural315>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural337 { pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural313 { r#Diff(Structural314),r#Full(Structural337), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural306 { pub r#address: Structural307,pub r#commit_id: Option<u64>,pub r#from_version: u64,pub r#generation: u64,pub r#payload: Structural313,pub r#subscription_id: u64,pub r#value_id: String,pub r#version: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableSubscriptionResult1Type { pub r#generation: u64,pub r#initial: Option<Structural306>,pub r#subscription_id: u64, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableSubscriptionResult1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-subscription-result@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableUnsubscribeInput1Type { pub r#generation: u64,pub r#subscription_id: u64, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableUnsubscribeInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-unsubscribe-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableValue1Type { pub r#value: phenix_core::PhenixValue,pub r#value_id: String,pub r#version: u64, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableValue1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-value@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypePageInput1Type { pub r#cursor: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypePageInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.page-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypePermissionRequest1Type { pub r#call_id: String,pub r#description: String,pub r#execution_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypePermissionRequest1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.permission-request@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypePermissionResponse1Type { r#AllowOnce,r#Cancelled,r#Deny, }
impl phenix_core::PhenixContract for PhenixApplicationTypePermissionResponse1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.permission-response@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural339 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural340 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural341 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural338 { r#Image(Structural339),r#Resource(Structural340),r#Text(Structural341), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypePromptInput1Type { pub r#content: Vec<Structural338>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypePromptInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.prompt-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural342 { r#Cancelled,r#EndTurn,r#MaxTokens,r#Refused, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypePromptResult1Type { pub r#execution_id: String,pub r#stop_reason: Structural342, }
impl phenix_core::PhenixContract for PhenixApplicationTypePromptResult1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.prompt-result@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeProvenance1Type { pub r#execution_id: String,pub r#inputs: Vec<String>,pub r#model_id: Option<String>,pub r#outputs: Vec<String>,pub r#routing_profile: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeProvenance1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.provenance@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeRoutingInfo1Type { pub r#id: String,pub r#name: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeRoutingInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.routing-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural343 { pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeRoutingProfiles1Type { pub r#available: Vec<Structural343>,pub r#selected: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeRoutingProfiles1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.routing-profiles@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeRoutingSelectInput1Type { pub r#profile_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeRoutingSelectInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.routing-select-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural346 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural345 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural346, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural344 { pub r#diagnostic: Structural345, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural349 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural354 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural355 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural356 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural357 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural358 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural359 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural360 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural361 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural362 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural363 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural364 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural365 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural366 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural367 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural353 { r#Cancelled,r#Closed,r#Conflict(Structural354),r#Disconnected,r#Failed(Structural355),r#InvalidInput(Structural356),r#InvalidPath(Structural357),r#InvalidResponse(Structural358),r#NotFound(Structural359),r#PermissionDenied(Structural360),r#SchemaMismatch(Structural361),r#StaleReference(Structural362),r#SubscriptionCapacity,r#TransactionConflict(Structural363),r#Unauthenticated(Structural364),r#UnknownValue(Structural365),r#UnsupportedCapability(Structural366),r#UnsupportedSnapshotPolicy(Structural367), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural352 { pub r#error: Structural353, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural351 { r#Cancelled,r#Completed,r#Failed(Structural352),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural350 { pub r#state: Structural351, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural368 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural371 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural372 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural373 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural374 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural375 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural376 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural377 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural378 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural379 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural380 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural381 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural382 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural383 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural384 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural370 { r#Cancelled,r#Closed,r#Conflict(Structural371),r#Disconnected,r#Failed(Structural372),r#InvalidInput(Structural373),r#InvalidPath(Structural374),r#InvalidResponse(Structural375),r#NotFound(Structural376),r#PermissionDenied(Structural377),r#SchemaMismatch(Structural378),r#StaleReference(Structural379),r#SubscriptionCapacity,r#TransactionConflict(Structural380),r#Unauthenticated(Structural381),r#UnknownValue(Structural382),r#UnsupportedCapability(Structural383),r#UnsupportedSnapshotPolicy(Structural384), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural369 { pub r#call_id: String,pub r#error: Structural370, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural385 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural348 { r#Progress(Structural349),r#State(Structural350),r#ToolCall(Structural368),r#ToolFailed(Structural369),r#ToolResult(Structural385), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural347 { pub r#execution_id: String,pub r#update: Structural348, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural389 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural390 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural391 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural388 { r#Image(Structural389),r#Resource(Structural390),r#Text(Structural391), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural392 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural387 { pub r#content: Vec<Structural388>,pub r#role: Structural392, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural386 { pub r#message: Structural387, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural393 { pub r#title: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural394 { pub r#execution_id: String,pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeSessionChange1Type { r#Closed,r#Diagnostic(Structural344),r#Execution(Structural347),r#Message(Structural386),r#Renamed(Structural393),r#TextDelta(Structural394), }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionChange1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-change@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionCreateInput1Type { pub r#title: Option<String>,pub r#working_directory: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionCreateInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-create-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionInfo1Type { pub r#session_id: String,pub r#title: Option<String>,pub r#working_directory: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionInput1Type { pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionLineage1Type { pub r#children: Vec<String>,pub r#parent: Option<String>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionLineage1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-lineage@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural395 { pub r#session_id: String,pub r#title: Option<String>,pub r#working_directory: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionList1Type { pub r#next_cursor: Option<String>,pub r#sessions: Vec<Structural395>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionList1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-list@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionRenameInput1Type { pub r#session_id: String,pub r#title: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionRenameInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-rename-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionResumeInput1Type { pub r#after_sequence: Option<u64>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionResumeInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-resume-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural396 { pub r#session_id: String,pub r#title: Option<String>,pub r#working_directory: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural401 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural400 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural401, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural399 { pub r#diagnostic: Structural400, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural404 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural409 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural410 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural411 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural412 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural413 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural414 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural415 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural416 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural417 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural418 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural419 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural420 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural421 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural422 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural408 { r#Cancelled,r#Closed,r#Conflict(Structural409),r#Disconnected,r#Failed(Structural410),r#InvalidInput(Structural411),r#InvalidPath(Structural412),r#InvalidResponse(Structural413),r#NotFound(Structural414),r#PermissionDenied(Structural415),r#SchemaMismatch(Structural416),r#StaleReference(Structural417),r#SubscriptionCapacity,r#TransactionConflict(Structural418),r#Unauthenticated(Structural419),r#UnknownValue(Structural420),r#UnsupportedCapability(Structural421),r#UnsupportedSnapshotPolicy(Structural422), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural407 { pub r#error: Structural408, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural406 { r#Cancelled,r#Completed,r#Failed(Structural407),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural405 { pub r#state: Structural406, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural423 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural426 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural427 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural428 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural429 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural430 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural431 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural432 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural433 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural434 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural435 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural436 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural437 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural438 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural439 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural425 { r#Cancelled,r#Closed,r#Conflict(Structural426),r#Disconnected,r#Failed(Structural427),r#InvalidInput(Structural428),r#InvalidPath(Structural429),r#InvalidResponse(Structural430),r#NotFound(Structural431),r#PermissionDenied(Structural432),r#SchemaMismatch(Structural433),r#StaleReference(Structural434),r#SubscriptionCapacity,r#TransactionConflict(Structural435),r#Unauthenticated(Structural436),r#UnknownValue(Structural437),r#UnsupportedCapability(Structural438),r#UnsupportedSnapshotPolicy(Structural439), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural424 { pub r#call_id: String,pub r#error: Structural425, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural440 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural403 { r#Progress(Structural404),r#State(Structural405),r#ToolCall(Structural423),r#ToolFailed(Structural424),r#ToolResult(Structural440), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural402 { pub r#execution_id: String,pub r#update: Structural403, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural444 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural445 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural446 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural443 { r#Image(Structural444),r#Resource(Structural445),r#Text(Structural446), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural447 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural442 { pub r#content: Vec<Structural443>,pub r#role: Structural447, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural441 { pub r#message: Structural442, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural448 { pub r#title: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural449 { pub r#execution_id: String,pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural398 { r#Closed,r#Diagnostic(Structural399),r#Execution(Structural402),r#Message(Structural441),r#Renamed(Structural448),r#TextDelta(Structural449), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural397 { pub r#sequence: u64,pub r#session_id: String,pub r#update: Structural398, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionSnapshot1Type { pub r#session: Structural396,pub r#through_sequence: u64,pub r#updates: Vec<Structural397>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionSnapshot1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-snapshot@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural453 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural452 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural453, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural451 { pub r#diagnostic: Structural452, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural456 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural461 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural462 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural463 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural464 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural465 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural466 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural467 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural468 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural469 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural470 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural471 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural472 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural473 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural474 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural460 { r#Cancelled,r#Closed,r#Conflict(Structural461),r#Disconnected,r#Failed(Structural462),r#InvalidInput(Structural463),r#InvalidPath(Structural464),r#InvalidResponse(Structural465),r#NotFound(Structural466),r#PermissionDenied(Structural467),r#SchemaMismatch(Structural468),r#StaleReference(Structural469),r#SubscriptionCapacity,r#TransactionConflict(Structural470),r#Unauthenticated(Structural471),r#UnknownValue(Structural472),r#UnsupportedCapability(Structural473),r#UnsupportedSnapshotPolicy(Structural474), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural459 { pub r#error: Structural460, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural458 { r#Cancelled,r#Completed,r#Failed(Structural459),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural457 { pub r#state: Structural458, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural475 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural478 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural479 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural480 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural481 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural482 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural483 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural484 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural485 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural486 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural487 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural488 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural489 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural490 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural491 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural477 { r#Cancelled,r#Closed,r#Conflict(Structural478),r#Disconnected,r#Failed(Structural479),r#InvalidInput(Structural480),r#InvalidPath(Structural481),r#InvalidResponse(Structural482),r#NotFound(Structural483),r#PermissionDenied(Structural484),r#SchemaMismatch(Structural485),r#StaleReference(Structural486),r#SubscriptionCapacity,r#TransactionConflict(Structural487),r#Unauthenticated(Structural488),r#UnknownValue(Structural489),r#UnsupportedCapability(Structural490),r#UnsupportedSnapshotPolicy(Structural491), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural476 { pub r#call_id: String,pub r#error: Structural477, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural492 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural455 { r#Progress(Structural456),r#State(Structural457),r#ToolCall(Structural475),r#ToolFailed(Structural476),r#ToolResult(Structural492), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural454 { pub r#execution_id: String,pub r#update: Structural455, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural496 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural497 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural498 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural495 { r#Image(Structural496),r#Resource(Structural497),r#Text(Structural498), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural499 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural494 { pub r#content: Vec<Structural495>,pub r#role: Structural499, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural493 { pub r#message: Structural494, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural500 { pub r#title: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural501 { pub r#execution_id: String,pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural450 { r#Closed,r#Diagnostic(Structural451),r#Execution(Structural454),r#Message(Structural493),r#Renamed(Structural500),r#TextDelta(Structural501), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionUpdate1Type { pub r#sequence: u64,pub r#session_id: String,pub r#update: Structural450, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionUpdate1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-update@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeSeverity1Type { r#Error,r#Info,r#Warning, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSeverity1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.severity@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSkillActivateInput1Type { pub r#session_id: String,pub r#skill_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSkillActivateInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.skill-activate-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSkillInfo1Type { pub r#active: bool,pub r#description: String,pub r#id: String,pub r#name: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSkillInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.skill-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural502 { pub r#active: bool,pub r#description: String,pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSkills1Type { pub r#skills: Vec<Structural502>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSkills1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.skills@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeStopReason1Type { r#Cancelled,r#EndTurn,r#MaxTokens,r#Refused, }
impl phenix_core::PhenixContract for PhenixApplicationTypeStopReason1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.stop-reason@1").expect("generated contract id is valid") } }
pub fn type_schemas() -> std::collections::BTreeMap<phenix_core::ContractId, phenix_core::PhenixSchema> { std::collections::BTreeMap::from([
(<PhenixApplicationError1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationError1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeAcknowledged1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeAcknowledged1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeAuthenticateInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeAuthenticateInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeAuthenticationMethod1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeAuthenticationMethod1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeAuthenticationMethods1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeAuthenticationMethods1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeAuthenticationResult1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeAuthenticationResult1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeCallableInfo1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeCallableInfo1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeCallableInvokeInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeCallableInvokeInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeCallableResult1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeCallableResult1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeCallables1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeCallables1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeCapabilityList1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeCapabilityList1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeClientCallableRequest1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeClientCallableRequest1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeClientCallableResponse1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeClientCallableResponse1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeContent1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeContent1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeDiagnostic1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeDiagnostic1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeDiagnostics1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeDiagnostics1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeElicitationRequest1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeElicitationRequest1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeElicitationResponse1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeElicitationResponse1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeEmpty1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeEmpty1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeExecutionChange1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeExecutionChange1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeExecutionInfo1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeExecutionInfo1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeExecutionInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeExecutionInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeExecutionState1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeExecutionState1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeExecutionTree1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeExecutionTree1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeExecutionUpdate1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeExecutionUpdate1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeMessageRole1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeMessageRole1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeMessage1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeMessage1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeModelInfo1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeModelInfo1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeModelSelectInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeModelSelectInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeModels1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeModels1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableAddress1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableAddress1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableChange1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableChange1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableDelivery1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableDelivery1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableGetInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableGetInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableInitial1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableInitial1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableList1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableList1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableMode1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableMode1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservablePathSegment1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservablePathSegment1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservablePath1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservablePath1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservablePayload1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservablePayload1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableRemove1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableRemove1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableReplace1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableReplace1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableResource1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableResource1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableScope1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableScope1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableSnapshotPolicy1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableSnapshotPolicy1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableSplice1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableSplice1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableSubscribeInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableSubscribeInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableSubscriptionResult1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableSubscriptionResult1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableUnsubscribeInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableUnsubscribeInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeObservableValue1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeObservableValue1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypePageInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypePageInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypePermissionRequest1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypePermissionRequest1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypePermissionResponse1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypePermissionResponse1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypePromptInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypePromptInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypePromptResult1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypePromptResult1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeProvenance1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeProvenance1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeRoutingInfo1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeRoutingInfo1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeRoutingProfiles1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeRoutingProfiles1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeRoutingSelectInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeRoutingSelectInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionChange1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionChange1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionCreateInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionCreateInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionInfo1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionInfo1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionLineage1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionLineage1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionList1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionList1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionRenameInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionRenameInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionResumeInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionResumeInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionSnapshot1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionSnapshot1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSessionUpdate1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSessionUpdate1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSeverity1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSeverity1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSkillActivateInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSkillActivateInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSkillInfo1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSkillInfo1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeSkills1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSkills1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeStopReason1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeStopReason1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
]) }
pub struct PhenixApplicationAuthenticate1Operation;
impl phenix_application_interface::Operation for PhenixApplicationAuthenticate1Operation { const ID: &'static str = "phenix.application.authenticate@1"; const CAPABILITY: &'static str = "phenix.application.capability.authentication@1"; type Input = PhenixApplicationTypeAuthenticateInput1Type; type Output = PhenixApplicationTypeAuthenticationResult1Type; }
impl PhenixApplicationAuthenticate1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeAuthenticateInput1Type) -> Result<PhenixApplicationTypeAuthenticationResult1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationAuthenticationList1Operation;
impl phenix_application_interface::Operation for PhenixApplicationAuthenticationList1Operation { const ID: &'static str = "phenix.application.authentication-list@1"; const CAPABILITY: &'static str = "phenix.application.capability.authentication@1"; type Input = PhenixApplicationTypeEmpty1Type; type Output = PhenixApplicationTypeAuthenticationMethods1Type; }
impl PhenixApplicationAuthenticationList1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeEmpty1Type) -> Result<PhenixApplicationTypeAuthenticationMethods1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationCallableInvoke1Operation;
impl phenix_application_interface::Operation for PhenixApplicationCallableInvoke1Operation { const ID: &'static str = "phenix.application.callable-invoke@1"; const CAPABILITY: &'static str = "phenix.application.capability.callables@1"; type Input = PhenixApplicationTypeCallableInvokeInput1Type; type Output = PhenixApplicationTypeCallableResult1Type; }
impl PhenixApplicationCallableInvoke1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeCallableInvokeInput1Type) -> Result<PhenixApplicationTypeCallableResult1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationCallableList1Operation;
impl phenix_application_interface::Operation for PhenixApplicationCallableList1Operation { const ID: &'static str = "phenix.application.callable-list@1"; const CAPABILITY: &'static str = "phenix.application.capability.callables@1"; type Input = PhenixApplicationTypeSessionInput1Type; type Output = PhenixApplicationTypeCallables1Type; }
impl PhenixApplicationCallableList1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionInput1Type) -> Result<PhenixApplicationTypeCallables1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationCancel1Operation;
impl phenix_application_interface::Operation for PhenixApplicationCancel1Operation { const ID: &'static str = "phenix.application.cancel@1"; const CAPABILITY: &'static str = "phenix.application.capability.prompt@1"; type Input = PhenixApplicationTypeSessionInput1Type; type Output = PhenixApplicationTypeAcknowledged1Type; }
impl PhenixApplicationCancel1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionInput1Type) -> Result<PhenixApplicationTypeAcknowledged1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationCapabilities1Operation;
impl phenix_application_interface::Operation for PhenixApplicationCapabilities1Operation { const ID: &'static str = "phenix.application.capabilities@1"; const CAPABILITY: &'static str = "phenix.application.capability.discovery@1"; type Input = PhenixApplicationTypeEmpty1Type; type Output = PhenixApplicationTypeCapabilityList1Type; }
impl PhenixApplicationCapabilities1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeEmpty1Type) -> Result<PhenixApplicationTypeCapabilityList1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationDiagnostics1Operation;
impl phenix_application_interface::Operation for PhenixApplicationDiagnostics1Operation { const ID: &'static str = "phenix.application.diagnostics@1"; const CAPABILITY: &'static str = "phenix.application.capability.diagnostics@1"; type Input = PhenixApplicationTypeEmpty1Type; type Output = PhenixApplicationTypeDiagnostics1Type; }
impl PhenixApplicationDiagnostics1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeEmpty1Type) -> Result<PhenixApplicationTypeDiagnostics1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationExecutionProvenance1Operation;
impl phenix_application_interface::Operation for PhenixApplicationExecutionProvenance1Operation { const ID: &'static str = "phenix.application.execution-provenance@1"; const CAPABILITY: &'static str = "phenix.application.capability.inspection@1"; type Input = PhenixApplicationTypeExecutionInput1Type; type Output = PhenixApplicationTypeProvenance1Type; }
impl PhenixApplicationExecutionProvenance1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeExecutionInput1Type) -> Result<PhenixApplicationTypeProvenance1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationExecutionTree1Operation;
impl phenix_application_interface::Operation for PhenixApplicationExecutionTree1Operation { const ID: &'static str = "phenix.application.execution-tree@1"; const CAPABILITY: &'static str = "phenix.application.capability.inspection@1"; type Input = PhenixApplicationTypeSessionInput1Type; type Output = PhenixApplicationTypeExecutionTree1Type; }
impl PhenixApplicationExecutionTree1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionInput1Type) -> Result<PhenixApplicationTypeExecutionTree1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationModelList1Operation;
impl phenix_application_interface::Operation for PhenixApplicationModelList1Operation { const ID: &'static str = "phenix.application.model-list@1"; const CAPABILITY: &'static str = "phenix.application.capability.models@1"; type Input = PhenixApplicationTypeSessionInput1Type; type Output = PhenixApplicationTypeModels1Type; }
impl PhenixApplicationModelList1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionInput1Type) -> Result<PhenixApplicationTypeModels1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationModelSelect1Operation;
impl phenix_application_interface::Operation for PhenixApplicationModelSelect1Operation { const ID: &'static str = "phenix.application.model-select@1"; const CAPABILITY: &'static str = "phenix.application.capability.models@1"; type Input = PhenixApplicationTypeModelSelectInput1Type; type Output = PhenixApplicationTypeModels1Type; }
impl PhenixApplicationModelSelect1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeModelSelectInput1Type) -> Result<PhenixApplicationTypeModels1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationObservableGet1Operation;
impl phenix_application_interface::Operation for PhenixApplicationObservableGet1Operation { const ID: &'static str = "phenix.application.observable-get@1"; const CAPABILITY: &'static str = "phenix.application.capability.observables@1"; type Input = PhenixApplicationTypeObservableGetInput1Type; type Output = PhenixApplicationTypeObservableValue1Type; }
impl PhenixApplicationObservableGet1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeObservableGetInput1Type) -> Result<PhenixApplicationTypeObservableValue1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationObservableList1Operation;
impl phenix_application_interface::Operation for PhenixApplicationObservableList1Operation { const ID: &'static str = "phenix.application.observable-list@1"; const CAPABILITY: &'static str = "phenix.application.capability.observables@1"; type Input = PhenixApplicationTypeEmpty1Type; type Output = PhenixApplicationTypeObservableList1Type; }
impl PhenixApplicationObservableList1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeEmpty1Type) -> Result<PhenixApplicationTypeObservableList1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationObservableSubscribe1Operation;
impl phenix_application_interface::Operation for PhenixApplicationObservableSubscribe1Operation { const ID: &'static str = "phenix.application.observable-subscribe@1"; const CAPABILITY: &'static str = "phenix.application.capability.observables@1"; type Input = PhenixApplicationTypeObservableSubscribeInput1Type; type Output = PhenixApplicationTypeObservableSubscriptionResult1Type; }
impl PhenixApplicationObservableSubscribe1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeObservableSubscribeInput1Type) -> Result<PhenixApplicationTypeObservableSubscriptionResult1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationObservableUnsubscribe1Operation;
impl phenix_application_interface::Operation for PhenixApplicationObservableUnsubscribe1Operation { const ID: &'static str = "phenix.application.observable-unsubscribe@1"; const CAPABILITY: &'static str = "phenix.application.capability.observables@1"; type Input = PhenixApplicationTypeObservableUnsubscribeInput1Type; type Output = PhenixApplicationTypeAcknowledged1Type; }
impl PhenixApplicationObservableUnsubscribe1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeObservableUnsubscribeInput1Type) -> Result<PhenixApplicationTypeAcknowledged1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationPrompt1Operation;
impl phenix_application_interface::Operation for PhenixApplicationPrompt1Operation { const ID: &'static str = "phenix.application.prompt@1"; const CAPABILITY: &'static str = "phenix.application.capability.prompt@1"; type Input = PhenixApplicationTypePromptInput1Type; type Output = PhenixApplicationTypePromptResult1Type; }
impl PhenixApplicationPrompt1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypePromptInput1Type) -> Result<PhenixApplicationTypePromptResult1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationRoutingList1Operation;
impl phenix_application_interface::Operation for PhenixApplicationRoutingList1Operation { const ID: &'static str = "phenix.application.routing-list@1"; const CAPABILITY: &'static str = "phenix.application.capability.routing@1"; type Input = PhenixApplicationTypeSessionInput1Type; type Output = PhenixApplicationTypeRoutingProfiles1Type; }
impl PhenixApplicationRoutingList1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionInput1Type) -> Result<PhenixApplicationTypeRoutingProfiles1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationRoutingSelect1Operation;
impl phenix_application_interface::Operation for PhenixApplicationRoutingSelect1Operation { const ID: &'static str = "phenix.application.routing-select@1"; const CAPABILITY: &'static str = "phenix.application.capability.routing@1"; type Input = PhenixApplicationTypeRoutingSelectInput1Type; type Output = PhenixApplicationTypeRoutingProfiles1Type; }
impl PhenixApplicationRoutingSelect1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeRoutingSelectInput1Type) -> Result<PhenixApplicationTypeRoutingProfiles1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationSessionClose1Operation;
impl phenix_application_interface::Operation for PhenixApplicationSessionClose1Operation { const ID: &'static str = "phenix.application.session-close@1"; const CAPABILITY: &'static str = "phenix.application.capability.sessions@1"; type Input = PhenixApplicationTypeSessionInput1Type; type Output = PhenixApplicationTypeAcknowledged1Type; }
impl PhenixApplicationSessionClose1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionInput1Type) -> Result<PhenixApplicationTypeAcknowledged1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationSessionCreate1Operation;
impl phenix_application_interface::Operation for PhenixApplicationSessionCreate1Operation { const ID: &'static str = "phenix.application.session-create@1"; const CAPABILITY: &'static str = "phenix.application.capability.sessions@1"; type Input = PhenixApplicationTypeSessionCreateInput1Type; type Output = PhenixApplicationTypeSessionInfo1Type; }
impl PhenixApplicationSessionCreate1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionCreateInput1Type) -> Result<PhenixApplicationTypeSessionInfo1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationSessionLineage1Operation;
impl phenix_application_interface::Operation for PhenixApplicationSessionLineage1Operation { const ID: &'static str = "phenix.application.session-lineage@1"; const CAPABILITY: &'static str = "phenix.application.capability.lineage@1"; type Input = PhenixApplicationTypeSessionInput1Type; type Output = PhenixApplicationTypeSessionLineage1Type; }
impl PhenixApplicationSessionLineage1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionInput1Type) -> Result<PhenixApplicationTypeSessionLineage1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationSessionList1Operation;
impl phenix_application_interface::Operation for PhenixApplicationSessionList1Operation { const ID: &'static str = "phenix.application.session-list@1"; const CAPABILITY: &'static str = "phenix.application.capability.session-list@1"; type Input = PhenixApplicationTypePageInput1Type; type Output = PhenixApplicationTypeSessionList1Type; }
impl PhenixApplicationSessionList1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypePageInput1Type) -> Result<PhenixApplicationTypeSessionList1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationSessionRename1Operation;
impl phenix_application_interface::Operation for PhenixApplicationSessionRename1Operation { const ID: &'static str = "phenix.application.session-rename@1"; const CAPABILITY: &'static str = "phenix.application.capability.session-rename@1"; type Input = PhenixApplicationTypeSessionRenameInput1Type; type Output = PhenixApplicationTypeSessionInfo1Type; }
impl PhenixApplicationSessionRename1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionRenameInput1Type) -> Result<PhenixApplicationTypeSessionInfo1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationSessionResume1Operation;
impl phenix_application_interface::Operation for PhenixApplicationSessionResume1Operation { const ID: &'static str = "phenix.application.session-resume@1"; const CAPABILITY: &'static str = "phenix.application.capability.session-resume@1"; type Input = PhenixApplicationTypeSessionResumeInput1Type; type Output = PhenixApplicationTypeSessionSnapshot1Type; }
impl PhenixApplicationSessionResume1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionResumeInput1Type) -> Result<PhenixApplicationTypeSessionSnapshot1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationSkillActivate1Operation;
impl phenix_application_interface::Operation for PhenixApplicationSkillActivate1Operation { const ID: &'static str = "phenix.application.skill-activate@1"; const CAPABILITY: &'static str = "phenix.application.capability.skills@1"; type Input = PhenixApplicationTypeSkillActivateInput1Type; type Output = PhenixApplicationTypeSkills1Type; }
impl PhenixApplicationSkillActivate1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSkillActivateInput1Type) -> Result<PhenixApplicationTypeSkills1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub struct PhenixApplicationSkillList1Operation;
impl phenix_application_interface::Operation for PhenixApplicationSkillList1Operation { const ID: &'static str = "phenix.application.skill-list@1"; const CAPABILITY: &'static str = "phenix.application.capability.skills@1"; type Input = PhenixApplicationTypeSessionInput1Type; type Output = PhenixApplicationTypeSkills1Type; }
impl PhenixApplicationSkillList1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeSessionInput1Type) -> Result<PhenixApplicationTypeSkills1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
pub type PhenixApplicationExecutionUpdate1Event = PhenixApplicationTypeExecutionUpdate1Type;
pub const PHENIXAPPLICATIONEXECUTIONUPDATE1EVENT: &str = "phenix.application.execution-update@1";
pub type PhenixApplicationObservableUpdate1Event = PhenixApplicationTypeObservableDelivery1Type;
pub const PHENIXAPPLICATIONOBSERVABLEUPDATE1EVENT: &str = "phenix.application.observable-update@1";
pub type PhenixApplicationSessionUpdate1Event = PhenixApplicationTypeSessionUpdate1Type;
pub const PHENIXAPPLICATIONSESSIONUPDATE1EVENT: &str = "phenix.application.session-update@1";
pub type PhenixApplicationClientCallable1CallbackRequest = PhenixApplicationTypeClientCallableRequest1Type;
pub const PHENIXAPPLICATIONCLIENTCALLABLE1CALLBACKREQUEST: &str = "phenix.application.client-callable@1";
pub type PhenixApplicationClientCallable1CallbackResponse = PhenixApplicationTypeClientCallableResponse1Type;
pub const PHENIXAPPLICATIONCLIENTCALLABLE1CALLBACKRESPONSE: &str = "phenix.application.client-callable@1";
pub type PhenixApplicationElicitation1CallbackRequest = PhenixApplicationTypeElicitationRequest1Type;
pub const PHENIXAPPLICATIONELICITATION1CALLBACKREQUEST: &str = "phenix.application.elicitation@1";
pub type PhenixApplicationElicitation1CallbackResponse = PhenixApplicationTypeElicitationResponse1Type;
pub const PHENIXAPPLICATIONELICITATION1CALLBACKRESPONSE: &str = "phenix.application.elicitation@1";
pub type PhenixApplicationPermission1CallbackRequest = PhenixApplicationTypePermissionRequest1Type;
pub const PHENIXAPPLICATIONPERMISSION1CALLBACKREQUEST: &str = "phenix.application.permission@1";
pub type PhenixApplicationPermission1CallbackResponse = PhenixApplicationTypePermissionResponse1Type;
pub const PHENIXAPPLICATIONPERMISSION1CALLBACKRESPONSE: &str = "phenix.application.permission@1";
pub struct PhenixApplicationCapabilityAuthentication1Capability; impl PhenixApplicationCapabilityAuthentication1Capability { pub const ID: &'static str = "phenix.application.capability.authentication@1"; }
pub struct PhenixApplicationCapabilityCallables1Capability; impl PhenixApplicationCapabilityCallables1Capability { pub const ID: &'static str = "phenix.application.capability.callables@1"; }
pub struct PhenixApplicationCapabilityClientCallables1Capability; impl PhenixApplicationCapabilityClientCallables1Capability { pub const ID: &'static str = "phenix.application.capability.client-callables@1"; }
pub struct PhenixApplicationCapabilityDiagnostics1Capability; impl PhenixApplicationCapabilityDiagnostics1Capability { pub const ID: &'static str = "phenix.application.capability.diagnostics@1"; }
pub struct PhenixApplicationCapabilityDiscovery1Capability; impl PhenixApplicationCapabilityDiscovery1Capability { pub const ID: &'static str = "phenix.application.capability.discovery@1"; }
pub struct PhenixApplicationCapabilityElicitation1Capability; impl PhenixApplicationCapabilityElicitation1Capability { pub const ID: &'static str = "phenix.application.capability.elicitation@1"; }
pub struct PhenixApplicationCapabilityInspection1Capability; impl PhenixApplicationCapabilityInspection1Capability { pub const ID: &'static str = "phenix.application.capability.inspection@1"; }
pub struct PhenixApplicationCapabilityLineage1Capability; impl PhenixApplicationCapabilityLineage1Capability { pub const ID: &'static str = "phenix.application.capability.lineage@1"; }
pub struct PhenixApplicationCapabilityModels1Capability; impl PhenixApplicationCapabilityModels1Capability { pub const ID: &'static str = "phenix.application.capability.models@1"; }
pub struct PhenixApplicationCapabilityObservables1Capability; impl PhenixApplicationCapabilityObservables1Capability { pub const ID: &'static str = "phenix.application.capability.observables@1"; }
pub struct PhenixApplicationCapabilityPermission1Capability; impl PhenixApplicationCapabilityPermission1Capability { pub const ID: &'static str = "phenix.application.capability.permission@1"; }
pub struct PhenixApplicationCapabilityPrompt1Capability; impl PhenixApplicationCapabilityPrompt1Capability { pub const ID: &'static str = "phenix.application.capability.prompt@1"; }
pub struct PhenixApplicationCapabilityRouting1Capability; impl PhenixApplicationCapabilityRouting1Capability { pub const ID: &'static str = "phenix.application.capability.routing@1"; }
pub struct PhenixApplicationCapabilitySessionList1Capability; impl PhenixApplicationCapabilitySessionList1Capability { pub const ID: &'static str = "phenix.application.capability.session-list@1"; }
pub struct PhenixApplicationCapabilitySessionRename1Capability; impl PhenixApplicationCapabilitySessionRename1Capability { pub const ID: &'static str = "phenix.application.capability.session-rename@1"; }
pub struct PhenixApplicationCapabilitySessionResume1Capability; impl PhenixApplicationCapabilitySessionResume1Capability { pub const ID: &'static str = "phenix.application.capability.session-resume@1"; }
pub struct PhenixApplicationCapabilitySessions1Capability; impl PhenixApplicationCapabilitySessions1Capability { pub const ID: &'static str = "phenix.application.capability.sessions@1"; }
pub struct PhenixApplicationCapabilitySkills1Capability; impl PhenixApplicationCapabilitySkills1Capability { pub const ID: &'static str = "phenix.application.capability.skills@1"; }
