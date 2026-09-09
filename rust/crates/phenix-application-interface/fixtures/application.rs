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
pub struct PhenixApplicationTypeCapabilityInvokeInput1Type { pub r#callable: phenix_core::PhenixValue,pub r#input: phenix_core::PhenixValue, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCapabilityInvokeInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.capability-invoke-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeCapabilityInvokeResult1Type { pub r#output: phenix_core::PhenixValue, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCapabilityInvokeResult1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.capability-invoke-result@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeCapabilityList1Type { pub r#capabilities: Vec<String>,pub r#interface: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCapabilityList1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.capability-list@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural17 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural18 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural19 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeContent1Type { r#Image(Structural17),r#Resource(Structural18),r#Text(Structural19), }
impl phenix_core::PhenixContract for PhenixApplicationTypeContent1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.content@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural20 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeDiagnostic1Type { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural20, }
impl phenix_core::PhenixContract for PhenixApplicationTypeDiagnostic1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.diagnostic@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural22 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural21 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural22, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeDiagnostics1Type { pub r#diagnostics: Vec<Structural21>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeDiagnostics1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.diagnostics@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeElicitationRequest1Type { pub r#message: String,pub r#schema: phenix_core::PhenixValue,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeElicitationRequest1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.elicitation-request@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural23 { pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeElicitationResponse1Type { r#Accepted(Structural23),r#Cancelled,r#Declined, }
impl phenix_core::PhenixContract for PhenixApplicationTypeElicitationResponse1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.elicitation-response@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeEmpty1Type {  }
impl phenix_core::PhenixContract for PhenixApplicationTypeEmpty1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.empty@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural24 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural29 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural30 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural31 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural32 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural33 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural34 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural35 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural36 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural37 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural38 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural39 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural40 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural41 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural42 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural28 { r#Cancelled,r#Closed,r#Conflict(Structural29),r#Disconnected,r#Failed(Structural30),r#InvalidInput(Structural31),r#InvalidPath(Structural32),r#InvalidResponse(Structural33),r#NotFound(Structural34),r#PermissionDenied(Structural35),r#SchemaMismatch(Structural36),r#StaleReference(Structural37),r#SubscriptionCapacity,r#TransactionConflict(Structural38),r#Unauthenticated(Structural39),r#UnknownValue(Structural40),r#UnsupportedCapability(Structural41),r#UnsupportedSnapshotPolicy(Structural42), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural27 { pub r#error: Structural28, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural26 { r#Cancelled,r#Completed,r#Failed(Structural27),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural25 { pub r#state: Structural26, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural43 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
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
pub struct Structural44 { pub r#call_id: String,pub r#error: Structural45, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural60 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeExecutionChange1Type { r#Progress(Structural24),r#State(Structural25),r#ToolCall(Structural43),r#ToolFailed(Structural44),r#ToolResult(Structural60), }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionChange1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-change@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural64 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural65 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural66 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural67 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural68 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural69 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural70 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural71 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural72 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural73 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural74 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural75 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural76 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural77 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural63 { r#Cancelled,r#Closed,r#Conflict(Structural64),r#Disconnected,r#Failed(Structural65),r#InvalidInput(Structural66),r#InvalidPath(Structural67),r#InvalidResponse(Structural68),r#NotFound(Structural69),r#PermissionDenied(Structural70),r#SchemaMismatch(Structural71),r#StaleReference(Structural72),r#SubscriptionCapacity,r#TransactionConflict(Structural73),r#Unauthenticated(Structural74),r#UnknownValue(Structural75),r#UnsupportedCapability(Structural76),r#UnsupportedSnapshotPolicy(Structural77), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural62 { pub r#error: Structural63, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural61 { r#Cancelled,r#Completed,r#Failed(Structural62),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionInfo1Type { pub r#execution_id: String,pub r#parent: Option<String>,pub r#state: Structural61, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionInput1Type { pub r#execution_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural80 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural81 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural82 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural83 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural84 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural85 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural86 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural87 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural88 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural89 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural90 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural91 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural92 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural93 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural79 { r#Cancelled,r#Closed,r#Conflict(Structural80),r#Disconnected,r#Failed(Structural81),r#InvalidInput(Structural82),r#InvalidPath(Structural83),r#InvalidResponse(Structural84),r#NotFound(Structural85),r#PermissionDenied(Structural86),r#SchemaMismatch(Structural87),r#StaleReference(Structural88),r#SubscriptionCapacity,r#TransactionConflict(Structural89),r#Unauthenticated(Structural90),r#UnknownValue(Structural91),r#UnsupportedCapability(Structural92),r#UnsupportedSnapshotPolicy(Structural93), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural78 { pub r#error: Structural79, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeExecutionState1Type { r#Cancelled,r#Completed,r#Failed(Structural78),r#Pending,r#Running, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionState1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-state@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural98 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural99 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural100 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural101 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural102 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural103 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural104 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural105 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural106 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural107 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural108 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural109 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural110 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural111 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural97 { r#Cancelled,r#Closed,r#Conflict(Structural98),r#Disconnected,r#Failed(Structural99),r#InvalidInput(Structural100),r#InvalidPath(Structural101),r#InvalidResponse(Structural102),r#NotFound(Structural103),r#PermissionDenied(Structural104),r#SchemaMismatch(Structural105),r#StaleReference(Structural106),r#SubscriptionCapacity,r#TransactionConflict(Structural107),r#Unauthenticated(Structural108),r#UnknownValue(Structural109),r#UnsupportedCapability(Structural110),r#UnsupportedSnapshotPolicy(Structural111), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural96 { pub r#error: Structural97, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural95 { r#Cancelled,r#Completed,r#Failed(Structural96),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural94 { pub r#execution_id: String,pub r#parent: Option<String>,pub r#state: Structural95, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionTree1Type { pub r#executions: Vec<Structural94>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionTree1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-tree@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural113 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural118 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural119 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural120 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural121 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural122 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural123 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural124 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural125 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural126 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural127 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural128 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural129 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural130 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural131 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural117 { r#Cancelled,r#Closed,r#Conflict(Structural118),r#Disconnected,r#Failed(Structural119),r#InvalidInput(Structural120),r#InvalidPath(Structural121),r#InvalidResponse(Structural122),r#NotFound(Structural123),r#PermissionDenied(Structural124),r#SchemaMismatch(Structural125),r#StaleReference(Structural126),r#SubscriptionCapacity,r#TransactionConflict(Structural127),r#Unauthenticated(Structural128),r#UnknownValue(Structural129),r#UnsupportedCapability(Structural130),r#UnsupportedSnapshotPolicy(Structural131), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural116 { pub r#error: Structural117, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural115 { r#Cancelled,r#Completed,r#Failed(Structural116),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural114 { pub r#state: Structural115, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural132 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
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
pub struct Structural133 { pub r#call_id: String,pub r#error: Structural134, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural149 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural112 { r#Progress(Structural113),r#State(Structural114),r#ToolCall(Structural132),r#ToolFailed(Structural133),r#ToolResult(Structural149), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionUpdate1Type { pub r#execution_id: String,pub r#sequence: u64,pub r#session_id: String,pub r#update: Structural112, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionUpdate1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-update@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeMessageRole1Type { r#Assistant,r#User, }
impl phenix_core::PhenixContract for PhenixApplicationTypeMessageRole1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.message-role@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural151 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural152 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural153 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural150 { r#Image(Structural151),r#Resource(Structural152),r#Text(Structural153), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural154 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeMessage1Type { pub r#content: Vec<Structural150>,pub r#role: Structural154, }
impl phenix_core::PhenixContract for PhenixApplicationTypeMessage1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.message@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeModelInfo1Type { pub r#description: Option<String>,pub r#id: String,pub r#name: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeModelInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.model-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeModelSelectInput1Type { pub r#model_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeModelSelectInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.model-select-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural155 { pub r#description: Option<String>,pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeModels1Type { pub r#available: Vec<Structural155>,pub r#selected: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeModels1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.models@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural158 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural159 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural160 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural157 { r#Field(Structural158),r#Index(Structural159),r#MapKey(Structural160),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural156 { pub r#segments: Vec<Structural157>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableAddress1Type { pub r#path: Structural156,pub r#value_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableAddress1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-address@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural165 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural166 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural167 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural164 { r#Field(Structural165),r#Index(Structural166),r#MapKey(Structural167),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural163 { pub r#segments: Vec<Structural164>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural162 { pub r#path: Structural163, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural161 { pub r#change: Structural162, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural172 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural173 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural174 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural171 { r#Field(Structural172),r#Index(Structural173),r#MapKey(Structural174),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural170 { pub r#segments: Vec<Structural171>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural169 { pub r#path: Structural170,pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural168 { pub r#change: Structural169, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural179 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural180 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural181 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural178 { r#Field(Structural179),r#Index(Structural180),r#MapKey(Structural181),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural177 { pub r#segments: Vec<Structural178>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural176 { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural177,pub r#start: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural175 { pub r#change: Structural176, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableChange1Type { r#Remove(Structural161),r#Replace(Structural168),r#Splice(Structural175), }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableChange1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-change@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural185 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural186 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural187 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural184 { r#Field(Structural185),r#Index(Structural186),r#MapKey(Structural187),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural183 { pub r#segments: Vec<Structural184>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural182 { pub r#path: Structural183,pub r#value_id: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural195 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural196 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural197 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural194 { r#Field(Structural195),r#Index(Structural196),r#MapKey(Structural197),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural193 { pub r#segments: Vec<Structural194>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural192 { pub r#path: Structural193, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural191 { pub r#change: Structural192, }
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
pub struct Structural199 { pub r#path: Structural200,pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural198 { pub r#change: Structural199, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural209 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural210 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural211 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural208 { r#Field(Structural209),r#Index(Structural210),r#MapKey(Structural211),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural207 { pub r#segments: Vec<Structural208>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural206 { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural207,pub r#start: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural205 { pub r#change: Structural206, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural190 { r#Remove(Structural191),r#Replace(Structural198),r#Splice(Structural205), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural189 { pub r#changes: Vec<Structural190>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural212 { pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural188 { r#Diff(Structural189),r#Full(Structural212), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableDelivery1Type { pub r#address: Structural182,pub r#commit_id: Option<u64>,pub r#from_version: u64,pub r#generation: u64,pub r#payload: Structural188,pub r#subscription_id: u64,pub r#value_id: String,pub r#version: u64, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableDelivery1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-delivery@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural215 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural216 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural217 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural214 { r#Field(Structural215),r#Index(Structural216),r#MapKey(Structural217),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural213 { pub r#segments: Vec<Structural214>, }
#[derive(Clone, Debug, PartialEq)]
pub struct Object218(pub phenix_core::ObjectRef);
impl phenix_core::ValueCodec for Object218 { fn phenix_type() -> phenix_core::PhenixSchema { phenix_core::PhenixSchema::Object { contract: phenix_core::ContractId::parse("phenix.observable@1").expect("generated object contract is valid") } } fn to_value(&self) -> phenix_core::PhenixValue { phenix_core::PhenixValue::Object(self.0.clone()) } fn from_value(value: &phenix_core::PhenixValue) -> Result<Self, phenix_core::ValueError> { <Self as phenix_core::ValueCodec>::phenix_type().parse(value)?; match value { phenix_core::PhenixValue::Object(reference) => Ok(Self(reference.clone())), _ => unreachable!("validated object value"), } } }
impl From<&Object218> for phenix_core::PhenixValue { fn from(value: &Object218) -> Self { <Object218 as phenix_core::ValueCodec>::to_value(value) } }
impl TryFrom<phenix_core::Exact<&phenix_core::PhenixValue>> for Object218 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Exact<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::from_value(value.0) } }
impl TryFrom<phenix_core::Project<&phenix_core::PhenixValue>> for Object218 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Project<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::project_from_value(value.0) } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableGetInput1Type { pub r#path: Structural213,pub r#reference: Object218, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableGetInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-get-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableInitial1Type { r#Full,r#None, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableInitial1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-initial@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural222 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural223 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural224 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural221 { r#Field(Structural222),r#Index(Structural223),r#MapKey(Structural224),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural220 { pub r#segments: Vec<Structural221>, }
#[derive(Clone, Debug, PartialEq)]
pub struct Object225(pub phenix_core::ObjectRef);
impl phenix_core::ValueCodec for Object225 { fn phenix_type() -> phenix_core::PhenixSchema { phenix_core::PhenixSchema::Object { contract: phenix_core::ContractId::parse("phenix.observable@1").expect("generated object contract is valid") } } fn to_value(&self) -> phenix_core::PhenixValue { phenix_core::PhenixValue::Object(self.0.clone()) } fn from_value(value: &phenix_core::PhenixValue) -> Result<Self, phenix_core::ValueError> { <Self as phenix_core::ValueCodec>::phenix_type().parse(value)?; match value { phenix_core::PhenixValue::Object(reference) => Ok(Self(reference.clone())), _ => unreachable!("validated object value"), } } }
impl From<&Object225> for phenix_core::PhenixValue { fn from(value: &Object225) -> Self { <Object225 as phenix_core::ValueCodec>::to_value(value) } }
impl TryFrom<phenix_core::Exact<&phenix_core::PhenixValue>> for Object225 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Exact<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::from_value(value.0) } }
impl TryFrom<phenix_core::Project<&phenix_core::PhenixValue>> for Object225 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Project<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::project_from_value(value.0) } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural226 { r#CopyOnChange,r#CurrentOnly, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural219 { pub r#binding_path: Vec<String>,pub r#namespace: String,pub r#path: Structural220,pub r#reference: Object225,pub r#resource: String,pub r#schema: phenix_core::PhenixValue,pub r#snapshot_policy: Structural226,pub r#value_id: String,pub r#version: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableList1Type { pub r#resources: Vec<Structural219>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableList1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-list@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableMode1Type { r#Diff,r#Full, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableMode1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-mode@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural227 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural228 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural229 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservablePathSegment1Type { r#Field(Structural227),r#Index(Structural228),r#MapKey(Structural229),r#OptionPayload,r#VariantPayload, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservablePathSegment1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-path-segment@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural231 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural232 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural233 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural230 { r#Field(Structural231),r#Index(Structural232),r#MapKey(Structural233),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservablePath1Type { pub r#segments: Vec<Structural230>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservablePath1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-path@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural240 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural241 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural242 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural239 { r#Field(Structural240),r#Index(Structural241),r#MapKey(Structural242),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural238 { pub r#segments: Vec<Structural239>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural237 { pub r#path: Structural238, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural236 { pub r#change: Structural237, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural247 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural248 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural249 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural246 { r#Field(Structural247),r#Index(Structural248),r#MapKey(Structural249),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural245 { pub r#segments: Vec<Structural246>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural244 { pub r#path: Structural245,pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural243 { pub r#change: Structural244, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural254 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural255 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural256 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural253 { r#Field(Structural254),r#Index(Structural255),r#MapKey(Structural256),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural252 { pub r#segments: Vec<Structural253>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural251 { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural252,pub r#start: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural250 { pub r#change: Structural251, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural235 { r#Remove(Structural236),r#Replace(Structural243),r#Splice(Structural250), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural234 { pub r#changes: Vec<Structural235>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural257 { pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservablePayload1Type { r#Diff(Structural234),r#Full(Structural257), }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservablePayload1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-payload@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural260 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural261 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural262 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural259 { r#Field(Structural260),r#Index(Structural261),r#MapKey(Structural262),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural258 { pub r#segments: Vec<Structural259>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableRemove1Type { pub r#path: Structural258, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableRemove1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-remove@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural265 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural266 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural267 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural264 { r#Field(Structural265),r#Index(Structural266),r#MapKey(Structural267),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural263 { pub r#segments: Vec<Structural264>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableReplace1Type { pub r#path: Structural263,pub r#value: phenix_core::PhenixValue, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableReplace1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-replace@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural270 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural271 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural272 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural269 { r#Field(Structural270),r#Index(Structural271),r#MapKey(Structural272),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural268 { pub r#segments: Vec<Structural269>, }
#[derive(Clone, Debug, PartialEq)]
pub struct Object273(pub phenix_core::ObjectRef);
impl phenix_core::ValueCodec for Object273 { fn phenix_type() -> phenix_core::PhenixSchema { phenix_core::PhenixSchema::Object { contract: phenix_core::ContractId::parse("phenix.observable@1").expect("generated object contract is valid") } } fn to_value(&self) -> phenix_core::PhenixValue { phenix_core::PhenixValue::Object(self.0.clone()) } fn from_value(value: &phenix_core::PhenixValue) -> Result<Self, phenix_core::ValueError> { <Self as phenix_core::ValueCodec>::phenix_type().parse(value)?; match value { phenix_core::PhenixValue::Object(reference) => Ok(Self(reference.clone())), _ => unreachable!("validated object value"), } } }
impl From<&Object273> for phenix_core::PhenixValue { fn from(value: &Object273) -> Self { <Object273 as phenix_core::ValueCodec>::to_value(value) } }
impl TryFrom<phenix_core::Exact<&phenix_core::PhenixValue>> for Object273 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Exact<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::from_value(value.0) } }
impl TryFrom<phenix_core::Project<&phenix_core::PhenixValue>> for Object273 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Project<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::project_from_value(value.0) } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural274 { r#CopyOnChange,r#CurrentOnly, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableResource1Type { pub r#binding_path: Vec<String>,pub r#namespace: String,pub r#path: Structural268,pub r#reference: Object273,pub r#resource: String,pub r#schema: phenix_core::PhenixValue,pub r#snapshot_policy: Structural274,pub r#value_id: String,pub r#version: u64, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableResource1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-resource@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableScope1Type { r#Exact,r#Recursive, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableScope1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-scope@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeObservableSnapshotPolicy1Type { r#CopyOnChange,r#CurrentOnly, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableSnapshotPolicy1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-snapshot-policy@1").expect("generated contract id is valid") } }
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
pub struct PhenixApplicationTypeObservableSplice1Type { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural275,pub r#start: u64, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableSplice1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-splice@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural280 { r#Full,r#None, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural281 { r#Diff,r#Full, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural284 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural285 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural286 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural283 { r#Field(Structural284),r#Index(Structural285),r#MapKey(Structural286),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural282 { pub r#segments: Vec<Structural283>, }
#[derive(Clone, Debug, PartialEq)]
pub struct Object287(pub phenix_core::ObjectRef);
impl phenix_core::ValueCodec for Object287 { fn phenix_type() -> phenix_core::PhenixSchema { phenix_core::PhenixSchema::Object { contract: phenix_core::ContractId::parse("phenix.observable@1").expect("generated object contract is valid") } } fn to_value(&self) -> phenix_core::PhenixValue { phenix_core::PhenixValue::Object(self.0.clone()) } fn from_value(value: &phenix_core::PhenixValue) -> Result<Self, phenix_core::ValueError> { <Self as phenix_core::ValueCodec>::phenix_type().parse(value)?; match value { phenix_core::PhenixValue::Object(reference) => Ok(Self(reference.clone())), _ => unreachable!("validated object value"), } } }
impl From<&Object287> for phenix_core::PhenixValue { fn from(value: &Object287) -> Self { <Object287 as phenix_core::ValueCodec>::to_value(value) } }
impl TryFrom<phenix_core::Exact<&phenix_core::PhenixValue>> for Object287 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Exact<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::from_value(value.0) } }
impl TryFrom<phenix_core::Project<&phenix_core::PhenixValue>> for Object287 { type Error = phenix_core::ValueError; fn try_from(value: phenix_core::Project<&phenix_core::PhenixValue>) -> Result<Self, Self::Error> { <Self as phenix_core::ValueCodec>::project_from_value(value.0) } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural288 { r#Exact,r#Recursive, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableSubscribeInput1Type { pub r#initial: Structural280,pub r#mode: Structural281,pub r#path: Structural282,pub r#reference: Object287,pub r#scope: Structural288, }
impl phenix_core::PhenixContract for PhenixApplicationTypeObservableSubscribeInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.observable-subscribe-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural293 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural294 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural295 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural292 { r#Field(Structural293),r#Index(Structural294),r#MapKey(Structural295),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural291 { pub r#segments: Vec<Structural292>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural290 { pub r#path: Structural291,pub r#value_id: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural303 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural304 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural305 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural302 { r#Field(Structural303),r#Index(Structural304),r#MapKey(Structural305),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural301 { pub r#segments: Vec<Structural302>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural300 { pub r#path: Structural301, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural299 { pub r#change: Structural300, }
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
pub struct Structural307 { pub r#path: Structural308,pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural306 { pub r#change: Structural307, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural317 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural318 { pub r#index: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural319 { pub r#key: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural316 { r#Field(Structural317),r#Index(Structural318),r#MapKey(Structural319),r#OptionPayload,r#VariantPayload, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural315 { pub r#segments: Vec<Structural316>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural314 { pub r#delete_count: u64,pub r#inserted: Vec<phenix_core::PhenixValue>,pub r#path: Structural315,pub r#start: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural313 { pub r#change: Structural314, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural298 { r#Remove(Structural299),r#Replace(Structural306),r#Splice(Structural313), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural297 { pub r#changes: Vec<Structural298>, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural320 { pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural296 { r#Diff(Structural297),r#Full(Structural320), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural289 { pub r#address: Structural290,pub r#commit_id: Option<u64>,pub r#from_version: u64,pub r#generation: u64,pub r#payload: Structural296,pub r#subscription_id: u64,pub r#value_id: String,pub r#version: u64, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeObservableSubscriptionResult1Type { pub r#generation: u64,pub r#initial: Option<Structural289>,pub r#subscription_id: u64, }
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
pub struct Structural322 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural323 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural324 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural321 { r#Image(Structural322),r#Resource(Structural323),r#Text(Structural324), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypePromptInput1Type { pub r#content: Vec<Structural321>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypePromptInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.prompt-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural325 { r#Cancelled,r#EndTurn,r#MaxTokens,r#Refused, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypePromptResult1Type { pub r#execution_id: String,pub r#stop_reason: Structural325, }
impl phenix_core::PhenixContract for PhenixApplicationTypePromptResult1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.prompt-result@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeProvenance1Type { pub r#execution_id: String,pub r#inputs: Vec<String>,pub r#model_id: Option<String>,pub r#outputs: Vec<String>,pub r#routing_profile: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeProvenance1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.provenance@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeRoutingInfo1Type { pub r#id: String,pub r#name: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeRoutingInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.routing-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural326 { pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeRoutingProfiles1Type { pub r#available: Vec<Structural326>,pub r#selected: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeRoutingProfiles1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.routing-profiles@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeRoutingSelectInput1Type { pub r#profile_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeRoutingSelectInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.routing-select-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSdkValue1Type { pub r#schema: phenix_core::PhenixValue,pub r#value: phenix_core::PhenixValue, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSdkValue1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.sdk-value@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural329 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural328 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural329, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural327 { pub r#diagnostic: Structural328, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural332 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural337 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural338 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural339 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural340 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural341 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural342 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural343 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural344 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural345 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural346 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural347 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural348 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural349 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural350 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural336 { r#Cancelled,r#Closed,r#Conflict(Structural337),r#Disconnected,r#Failed(Structural338),r#InvalidInput(Structural339),r#InvalidPath(Structural340),r#InvalidResponse(Structural341),r#NotFound(Structural342),r#PermissionDenied(Structural343),r#SchemaMismatch(Structural344),r#StaleReference(Structural345),r#SubscriptionCapacity,r#TransactionConflict(Structural346),r#Unauthenticated(Structural347),r#UnknownValue(Structural348),r#UnsupportedCapability(Structural349),r#UnsupportedSnapshotPolicy(Structural350), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural335 { pub r#error: Structural336, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural334 { r#Cancelled,r#Completed,r#Failed(Structural335),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural333 { pub r#state: Structural334, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural351 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
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
pub struct Structural352 { pub r#call_id: String,pub r#error: Structural353, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural368 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural331 { r#Progress(Structural332),r#State(Structural333),r#ToolCall(Structural351),r#ToolFailed(Structural352),r#ToolResult(Structural368), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural330 { pub r#execution_id: String,pub r#update: Structural331, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural372 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural373 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural374 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural371 { r#Image(Structural372),r#Resource(Structural373),r#Text(Structural374), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural375 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural370 { pub r#content: Vec<Structural371>,pub r#role: Structural375, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural369 { pub r#message: Structural370, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural376 { pub r#title: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural377 { pub r#execution_id: String,pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeSessionChange1Type { r#Closed,r#Diagnostic(Structural327),r#Execution(Structural330),r#Message(Structural369),r#Renamed(Structural376),r#TextDelta(Structural377), }
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
pub struct Structural378 { pub r#session_id: String,pub r#title: Option<String>,pub r#working_directory: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionList1Type { pub r#next_cursor: Option<String>,pub r#sessions: Vec<Structural378>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionList1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-list@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionRenameInput1Type { pub r#session_id: String,pub r#title: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionRenameInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-rename-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionResumeInput1Type { pub r#after_sequence: Option<u64>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionResumeInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-resume-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural379 { pub r#session_id: String,pub r#title: Option<String>,pub r#working_directory: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural384 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural383 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural384, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural382 { pub r#diagnostic: Structural383, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural387 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural392 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural393 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural394 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural395 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural396 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural397 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural398 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural399 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural400 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural401 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural402 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural403 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural404 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural405 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural391 { r#Cancelled,r#Closed,r#Conflict(Structural392),r#Disconnected,r#Failed(Structural393),r#InvalidInput(Structural394),r#InvalidPath(Structural395),r#InvalidResponse(Structural396),r#NotFound(Structural397),r#PermissionDenied(Structural398),r#SchemaMismatch(Structural399),r#StaleReference(Structural400),r#SubscriptionCapacity,r#TransactionConflict(Structural401),r#Unauthenticated(Structural402),r#UnknownValue(Structural403),r#UnsupportedCapability(Structural404),r#UnsupportedSnapshotPolicy(Structural405), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural390 { pub r#error: Structural391, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural389 { r#Cancelled,r#Completed,r#Failed(Structural390),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural388 { pub r#state: Structural389, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural406 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
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
pub struct Structural407 { pub r#call_id: String,pub r#error: Structural408, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural423 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural386 { r#Progress(Structural387),r#State(Structural388),r#ToolCall(Structural406),r#ToolFailed(Structural407),r#ToolResult(Structural423), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural385 { pub r#execution_id: String,pub r#update: Structural386, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural427 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural428 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural429 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural426 { r#Image(Structural427),r#Resource(Structural428),r#Text(Structural429), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural430 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural425 { pub r#content: Vec<Structural426>,pub r#role: Structural430, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural424 { pub r#message: Structural425, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural431 { pub r#title: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural432 { pub r#execution_id: String,pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural381 { r#Closed,r#Diagnostic(Structural382),r#Execution(Structural385),r#Message(Structural424),r#Renamed(Structural431),r#TextDelta(Structural432), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural380 { pub r#sequence: u64,pub r#session_id: String,pub r#update: Structural381, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionSnapshot1Type { pub r#session: Structural379,pub r#through_sequence: u64,pub r#updates: Vec<Structural380>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionSnapshot1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-snapshot@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural436 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural435 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural436, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural434 { pub r#diagnostic: Structural435, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural439 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural444 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural445 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural446 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural447 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural448 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural449 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural450 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural451 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural452 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural453 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural454 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural455 { pub r#value: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural456 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural457 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural443 { r#Cancelled,r#Closed,r#Conflict(Structural444),r#Disconnected,r#Failed(Structural445),r#InvalidInput(Structural446),r#InvalidPath(Structural447),r#InvalidResponse(Structural448),r#NotFound(Structural449),r#PermissionDenied(Structural450),r#SchemaMismatch(Structural451),r#StaleReference(Structural452),r#SubscriptionCapacity,r#TransactionConflict(Structural453),r#Unauthenticated(Structural454),r#UnknownValue(Structural455),r#UnsupportedCapability(Structural456),r#UnsupportedSnapshotPolicy(Structural457), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural442 { pub r#error: Structural443, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural441 { r#Cancelled,r#Completed,r#Failed(Structural442),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural440 { pub r#state: Structural441, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural458 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
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
pub struct Structural459 { pub r#call_id: String,pub r#error: Structural460, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural475 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural438 { r#Progress(Structural439),r#State(Structural440),r#ToolCall(Structural458),r#ToolFailed(Structural459),r#ToolResult(Structural475), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural437 { pub r#execution_id: String,pub r#update: Structural438, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural479 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural480 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural481 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural478 { r#Image(Structural479),r#Resource(Structural480),r#Text(Structural481), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural482 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural477 { pub r#content: Vec<Structural478>,pub r#role: Structural482, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural476 { pub r#message: Structural477, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural483 { pub r#title: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural484 { pub r#execution_id: String,pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural433 { r#Closed,r#Diagnostic(Structural434),r#Execution(Structural437),r#Message(Structural476),r#Renamed(Structural483),r#TextDelta(Structural484), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionUpdate1Type { pub r#sequence: u64,pub r#session_id: String,pub r#update: Structural433, }
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
pub struct Structural485 { pub r#active: bool,pub r#description: String,pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSkills1Type { pub r#skills: Vec<Structural485>, }
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
(<PhenixApplicationTypeCapabilityInvokeInput1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeCapabilityInvokeInput1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeCapabilityInvokeResult1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeCapabilityInvokeResult1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
(<PhenixApplicationTypeCapabilityList1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeCapabilityList1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
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
(<PhenixApplicationTypeSdkValue1Type as phenix_core::PhenixContract>::contract_id(), <PhenixApplicationTypeSdkValue1Type as phenix_core::HasPhenixSchema>::phenix_schema()),
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
pub struct PhenixApplicationCapabilityInvoke1Operation;
impl phenix_application_interface::Operation for PhenixApplicationCapabilityInvoke1Operation { const ID: &'static str = "phenix.application.capability-invoke@1"; const CAPABILITY: &'static str = "phenix.application.capability.capabilities@1"; type Input = PhenixApplicationTypeCapabilityInvokeInput1Type; type Output = PhenixApplicationTypeCapabilityInvokeResult1Type; }
impl PhenixApplicationCapabilityInvoke1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeCapabilityInvokeInput1Type) -> Result<PhenixApplicationTypeCapabilityInvokeResult1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
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
pub struct PhenixApplicationSdkGet1Operation;
impl phenix_application_interface::Operation for PhenixApplicationSdkGet1Operation { const ID: &'static str = "phenix.application.sdk-get@1"; const CAPABILITY: &'static str = "phenix.application.capability.sdk@1"; type Input = PhenixApplicationTypeEmpty1Type; type Output = PhenixApplicationTypeSdkValue1Type; }
impl PhenixApplicationSdkGet1Operation { pub async fn invoke<T: phenix_application_interface::ApplicationTransport>(client: &phenix_application_interface::ApplicationClient<T>, input: PhenixApplicationTypeEmpty1Type) -> Result<PhenixApplicationTypeSdkValue1Type, phenix_application_interface::types::ApplicationError> { client.invoke::<Self>(input).await } }
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
pub type PhenixApplicationCapabilityCall1CallbackRequest = PhenixApplicationTypeCapabilityInvokeInput1Type;
pub const PHENIXAPPLICATIONCAPABILITYCALL1CALLBACKREQUEST: &str = "phenix.application.capability-call@1";
pub type PhenixApplicationCapabilityCall1CallbackResponse = PhenixApplicationTypeCapabilityInvokeResult1Type;
pub const PHENIXAPPLICATIONCAPABILITYCALL1CALLBACKRESPONSE: &str = "phenix.application.capability-call@1";
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
pub struct PhenixApplicationCapabilityCapabilities1Capability; impl PhenixApplicationCapabilityCapabilities1Capability { pub const ID: &'static str = "phenix.application.capability.capabilities@1"; }
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
pub struct PhenixApplicationCapabilitySdk1Capability; impl PhenixApplicationCapabilitySdk1Capability { pub const ID: &'static str = "phenix.application.capability.sdk@1"; }
pub struct PhenixApplicationCapabilitySessionList1Capability; impl PhenixApplicationCapabilitySessionList1Capability { pub const ID: &'static str = "phenix.application.capability.session-list@1"; }
pub struct PhenixApplicationCapabilitySessionRename1Capability; impl PhenixApplicationCapabilitySessionRename1Capability { pub const ID: &'static str = "phenix.application.capability.session-rename@1"; }
pub struct PhenixApplicationCapabilitySessionResume1Capability; impl PhenixApplicationCapabilitySessionResume1Capability { pub const ID: &'static str = "phenix.application.capability.session-resume@1"; }
pub struct PhenixApplicationCapabilitySessions1Capability; impl PhenixApplicationCapabilitySessions1Capability { pub const ID: &'static str = "phenix.application.capability.sessions@1"; }
pub struct PhenixApplicationCapabilitySkills1Capability; impl PhenixApplicationCapabilitySkills1Capability { pub const ID: &'static str = "phenix.application.capability.skills@1"; }
