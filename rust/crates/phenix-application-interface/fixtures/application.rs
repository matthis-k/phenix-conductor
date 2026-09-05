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
pub struct Structural4 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural5 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural6 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural7 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationError1Type { r#Cancelled,r#Conflict(Structural0),r#Disconnected,r#Failed(Structural1),r#InvalidInput(Structural2),r#InvalidResponse(Structural3),r#NotFound(Structural4),r#PermissionDenied(Structural5),r#Unauthenticated(Structural6),r#UnsupportedCapability(Structural7), }
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
pub struct Structural8 { pub r#description: Option<String>,pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeAuthenticationMethods1Type { pub r#methods: Vec<Structural8>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeAuthenticationMethods1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.authentication-methods@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural9 { pub r#instructions: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeAuthenticationResult1Type { r#Authenticated,r#External(Structural9), }
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
pub struct Structural10 { pub r#description: String,pub r#id: String,pub r#input: phenix_core::PhenixValue,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeCallables1Type { pub r#callables: Vec<Structural10>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCallables1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.callables@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeCapabilityList1Type { pub r#capabilities: Vec<String>,pub r#interface: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeCapabilityList1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.capability-list@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeClientCallableRequest1Type { pub r#call_id: String,pub r#callable_id: String,pub r#execution_id: String,pub r#input: phenix_core::PhenixValue,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeClientCallableRequest1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.client-callable-request@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural11 { pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural14 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural15 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural16 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural17 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural18 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural19 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural20 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural21 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural13 { r#Cancelled,r#Conflict(Structural14),r#Disconnected,r#Failed(Structural15),r#InvalidInput(Structural16),r#InvalidResponse(Structural17),r#NotFound(Structural18),r#PermissionDenied(Structural19),r#Unauthenticated(Structural20),r#UnsupportedCapability(Structural21), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural12 { pub r#error: Structural13, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeClientCallableResponse1Type { r#Completed(Structural11),r#Failed(Structural12), }
impl phenix_core::PhenixContract for PhenixApplicationTypeClientCallableResponse1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.client-callable-response@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural22 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural23 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural24 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeContent1Type { r#Image(Structural22),r#Resource(Structural23),r#Text(Structural24), }
impl phenix_core::PhenixContract for PhenixApplicationTypeContent1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.content@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural25 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeDiagnostic1Type { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural25, }
impl phenix_core::PhenixContract for PhenixApplicationTypeDiagnostic1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.diagnostic@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural27 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural26 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural27, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeDiagnostics1Type { pub r#diagnostics: Vec<Structural26>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeDiagnostics1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.diagnostics@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeElicitationRequest1Type { pub r#message: String,pub r#schema: phenix_core::PhenixValue,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeElicitationRequest1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.elicitation-request@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural28 { pub r#value: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeElicitationResponse1Type { r#Accepted(Structural28),r#Cancelled,r#Declined, }
impl phenix_core::PhenixContract for PhenixApplicationTypeElicitationResponse1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.elicitation-response@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeEmpty1Type {  }
impl phenix_core::PhenixContract for PhenixApplicationTypeEmpty1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.empty@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural29 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural34 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural35 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural36 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural37 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural38 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural39 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural40 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural41 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural33 { r#Cancelled,r#Conflict(Structural34),r#Disconnected,r#Failed(Structural35),r#InvalidInput(Structural36),r#InvalidResponse(Structural37),r#NotFound(Structural38),r#PermissionDenied(Structural39),r#Unauthenticated(Structural40),r#UnsupportedCapability(Structural41), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural32 { pub r#error: Structural33, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural31 { r#Cancelled,r#Completed,r#Failed(Structural32),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural30 { pub r#state: Structural31, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural42 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural45 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural46 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural47 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural48 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural49 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural50 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural51 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural52 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural44 { r#Cancelled,r#Conflict(Structural45),r#Disconnected,r#Failed(Structural46),r#InvalidInput(Structural47),r#InvalidResponse(Structural48),r#NotFound(Structural49),r#PermissionDenied(Structural50),r#Unauthenticated(Structural51),r#UnsupportedCapability(Structural52), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural43 { pub r#call_id: String,pub r#error: Structural44, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural53 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeExecutionChange1Type { r#Progress(Structural29),r#State(Structural30),r#ToolCall(Structural42),r#ToolFailed(Structural43),r#ToolResult(Structural53), }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionChange1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-change@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural57 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural58 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural59 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural60 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural61 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural62 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural63 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural64 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural56 { r#Cancelled,r#Conflict(Structural57),r#Disconnected,r#Failed(Structural58),r#InvalidInput(Structural59),r#InvalidResponse(Structural60),r#NotFound(Structural61),r#PermissionDenied(Structural62),r#Unauthenticated(Structural63),r#UnsupportedCapability(Structural64), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural55 { pub r#error: Structural56, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural54 { r#Cancelled,r#Completed,r#Failed(Structural55),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionInfo1Type { pub r#execution_id: String,pub r#parent: Option<String>,pub r#state: Structural54, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionInput1Type { pub r#execution_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural67 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural68 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural69 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural70 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural71 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural72 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural73 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural74 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural66 { r#Cancelled,r#Conflict(Structural67),r#Disconnected,r#Failed(Structural68),r#InvalidInput(Structural69),r#InvalidResponse(Structural70),r#NotFound(Structural71),r#PermissionDenied(Structural72),r#Unauthenticated(Structural73),r#UnsupportedCapability(Structural74), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural65 { pub r#error: Structural66, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeExecutionState1Type { r#Cancelled,r#Completed,r#Failed(Structural65),r#Pending,r#Running, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionState1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-state@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural79 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural80 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural81 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural82 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural83 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural84 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural85 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural86 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural78 { r#Cancelled,r#Conflict(Structural79),r#Disconnected,r#Failed(Structural80),r#InvalidInput(Structural81),r#InvalidResponse(Structural82),r#NotFound(Structural83),r#PermissionDenied(Structural84),r#Unauthenticated(Structural85),r#UnsupportedCapability(Structural86), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural77 { pub r#error: Structural78, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural76 { r#Cancelled,r#Completed,r#Failed(Structural77),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural75 { pub r#execution_id: String,pub r#parent: Option<String>,pub r#state: Structural76, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionTree1Type { pub r#executions: Vec<Structural75>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionTree1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-tree@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural88 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural93 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural94 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural95 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural96 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural97 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural98 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural99 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural100 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural92 { r#Cancelled,r#Conflict(Structural93),r#Disconnected,r#Failed(Structural94),r#InvalidInput(Structural95),r#InvalidResponse(Structural96),r#NotFound(Structural97),r#PermissionDenied(Structural98),r#Unauthenticated(Structural99),r#UnsupportedCapability(Structural100), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural91 { pub r#error: Structural92, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural90 { r#Cancelled,r#Completed,r#Failed(Structural91),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural89 { pub r#state: Structural90, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural101 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural104 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural105 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural106 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural107 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural108 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural109 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural110 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural111 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural103 { r#Cancelled,r#Conflict(Structural104),r#Disconnected,r#Failed(Structural105),r#InvalidInput(Structural106),r#InvalidResponse(Structural107),r#NotFound(Structural108),r#PermissionDenied(Structural109),r#Unauthenticated(Structural110),r#UnsupportedCapability(Structural111), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural102 { pub r#call_id: String,pub r#error: Structural103, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural112 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural87 { r#Progress(Structural88),r#State(Structural89),r#ToolCall(Structural101),r#ToolFailed(Structural102),r#ToolResult(Structural112), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeExecutionUpdate1Type { pub r#execution_id: String,pub r#sequence: u64,pub r#session_id: String,pub r#update: Structural87, }
impl phenix_core::PhenixContract for PhenixApplicationTypeExecutionUpdate1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.execution-update@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeMessageRole1Type { r#Assistant,r#User, }
impl phenix_core::PhenixContract for PhenixApplicationTypeMessageRole1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.message-role@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural114 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural115 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural116 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural113 { r#Image(Structural114),r#Resource(Structural115),r#Text(Structural116), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural117 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeMessage1Type { pub r#content: Vec<Structural113>,pub r#role: Structural117, }
impl phenix_core::PhenixContract for PhenixApplicationTypeMessage1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.message@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeModelInfo1Type { pub r#description: Option<String>,pub r#id: String,pub r#name: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeModelInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.model-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeModelSelectInput1Type { pub r#model_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeModelSelectInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.model-select-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural118 { pub r#description: Option<String>,pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeModels1Type { pub r#available: Vec<Structural118>,pub r#selected: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeModels1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.models@1").expect("generated contract id is valid") } }
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
pub struct Structural120 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural121 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural122 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural119 { r#Image(Structural120),r#Resource(Structural121),r#Text(Structural122), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypePromptInput1Type { pub r#content: Vec<Structural119>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypePromptInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.prompt-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural123 { r#Cancelled,r#EndTurn,r#MaxTokens,r#Refused, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypePromptResult1Type { pub r#execution_id: String,pub r#stop_reason: Structural123, }
impl phenix_core::PhenixContract for PhenixApplicationTypePromptResult1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.prompt-result@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeProvenance1Type { pub r#execution_id: String,pub r#inputs: Vec<String>,pub r#model_id: Option<String>,pub r#outputs: Vec<String>,pub r#routing_profile: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeProvenance1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.provenance@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeRoutingInfo1Type { pub r#id: String,pub r#name: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeRoutingInfo1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.routing-info@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural124 { pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeRoutingProfiles1Type { pub r#available: Vec<Structural124>,pub r#selected: Option<String>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeRoutingProfiles1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.routing-profiles@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeRoutingSelectInput1Type { pub r#profile_id: String,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeRoutingSelectInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.routing-select-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural127 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural126 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural127, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural125 { pub r#diagnostic: Structural126, }
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
pub struct Structural139 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural140 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural141 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural142 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural134 { r#Cancelled,r#Conflict(Structural135),r#Disconnected,r#Failed(Structural136),r#InvalidInput(Structural137),r#InvalidResponse(Structural138),r#NotFound(Structural139),r#PermissionDenied(Structural140),r#Unauthenticated(Structural141),r#UnsupportedCapability(Structural142), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural133 { pub r#error: Structural134, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural132 { r#Cancelled,r#Completed,r#Failed(Structural133),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural131 { pub r#state: Structural132, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural143 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural146 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural147 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural148 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural149 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural150 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural151 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural152 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural153 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural145 { r#Cancelled,r#Conflict(Structural146),r#Disconnected,r#Failed(Structural147),r#InvalidInput(Structural148),r#InvalidResponse(Structural149),r#NotFound(Structural150),r#PermissionDenied(Structural151),r#Unauthenticated(Structural152),r#UnsupportedCapability(Structural153), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural144 { pub r#call_id: String,pub r#error: Structural145, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural154 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural129 { r#Progress(Structural130),r#State(Structural131),r#ToolCall(Structural143),r#ToolFailed(Structural144),r#ToolResult(Structural154), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural128 { pub r#execution_id: String,pub r#update: Structural129, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural158 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural159 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural160 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural157 { r#Image(Structural158),r#Resource(Structural159),r#Text(Structural160), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural161 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural156 { pub r#content: Vec<Structural157>,pub r#role: Structural161, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural155 { pub r#message: Structural156, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural162 { pub r#title: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural163 { pub r#execution_id: String,pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum PhenixApplicationTypeSessionChange1Type { r#Closed,r#Diagnostic(Structural125),r#Execution(Structural128),r#Message(Structural155),r#Renamed(Structural162),r#TextDelta(Structural163), }
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
pub struct Structural164 { pub r#session_id: String,pub r#title: Option<String>,pub r#working_directory: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionList1Type { pub r#next_cursor: Option<String>,pub r#sessions: Vec<Structural164>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionList1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-list@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionRenameInput1Type { pub r#session_id: String,pub r#title: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionRenameInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-rename-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionResumeInput1Type { pub r#after_sequence: Option<u64>,pub r#session_id: String, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionResumeInput1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-resume-input@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural165 { pub r#session_id: String,pub r#title: Option<String>,pub r#working_directory: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural170 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural169 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural170, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural168 { pub r#diagnostic: Structural169, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural173 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural178 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural179 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural180 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural181 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural182 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural183 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural184 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural185 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural177 { r#Cancelled,r#Conflict(Structural178),r#Disconnected,r#Failed(Structural179),r#InvalidInput(Structural180),r#InvalidResponse(Structural181),r#NotFound(Structural182),r#PermissionDenied(Structural183),r#Unauthenticated(Structural184),r#UnsupportedCapability(Structural185), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural176 { pub r#error: Structural177, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural175 { r#Cancelled,r#Completed,r#Failed(Structural176),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural174 { pub r#state: Structural175, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural186 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural189 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural190 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural191 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural192 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural193 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural194 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural195 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural196 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural188 { r#Cancelled,r#Conflict(Structural189),r#Disconnected,r#Failed(Structural190),r#InvalidInput(Structural191),r#InvalidResponse(Structural192),r#NotFound(Structural193),r#PermissionDenied(Structural194),r#Unauthenticated(Structural195),r#UnsupportedCapability(Structural196), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural187 { pub r#call_id: String,pub r#error: Structural188, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural197 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural172 { r#Progress(Structural173),r#State(Structural174),r#ToolCall(Structural186),r#ToolFailed(Structural187),r#ToolResult(Structural197), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural171 { pub r#execution_id: String,pub r#update: Structural172, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural201 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural202 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural203 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural200 { r#Image(Structural201),r#Resource(Structural202),r#Text(Structural203), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural204 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural199 { pub r#content: Vec<Structural200>,pub r#role: Structural204, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural198 { pub r#message: Structural199, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural205 { pub r#title: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural206 { pub r#execution_id: String,pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural167 { r#Closed,r#Diagnostic(Structural168),r#Execution(Structural171),r#Message(Structural198),r#Renamed(Structural205),r#TextDelta(Structural206), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural166 { pub r#sequence: u64,pub r#session_id: String,pub r#update: Structural167, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionSnapshot1Type { pub r#session: Structural165,pub r#through_sequence: u64,pub r#updates: Vec<Structural166>, }
impl phenix_core::PhenixContract for PhenixApplicationTypeSessionSnapshot1Type { fn contract_id() -> phenix_core::ContractId { phenix_core::ContractId::parse("phenix.application.type.session-snapshot@1").expect("generated contract id is valid") } }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural210 { r#Error,r#Info,r#Warning, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural209 { pub r#code: String,pub r#message: String,pub r#resource: Option<String>,pub r#severity: Structural210, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural208 { pub r#diagnostic: Structural209, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural213 { pub r#fraction: Option<f64>,pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural218 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural219 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural220 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural221 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural222 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural223 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural224 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural225 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural217 { r#Cancelled,r#Conflict(Structural218),r#Disconnected,r#Failed(Structural219),r#InvalidInput(Structural220),r#InvalidResponse(Structural221),r#NotFound(Structural222),r#PermissionDenied(Structural223),r#Unauthenticated(Structural224),r#UnsupportedCapability(Structural225), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural216 { pub r#error: Structural217, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural215 { r#Cancelled,r#Completed,r#Failed(Structural216),r#Pending,r#Running, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural214 { pub r#state: Structural215, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural226 { pub r#call_id: String,pub r#callable_id: String,pub r#input: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural229 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural230 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural231 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural232 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural233 { pub r#resource: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural234 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural235 { pub r#message: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural236 { pub r#capability: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural228 { r#Cancelled,r#Conflict(Structural229),r#Disconnected,r#Failed(Structural230),r#InvalidInput(Structural231),r#InvalidResponse(Structural232),r#NotFound(Structural233),r#PermissionDenied(Structural234),r#Unauthenticated(Structural235),r#UnsupportedCapability(Structural236), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural227 { pub r#call_id: String,pub r#error: Structural228, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural237 { pub r#call_id: String,pub r#output: phenix_core::PhenixValue, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural212 { r#Progress(Structural213),r#State(Structural214),r#ToolCall(Structural226),r#ToolFailed(Structural227),r#ToolResult(Structural237), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural211 { pub r#execution_id: String,pub r#update: Structural212, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural241 { pub r#data: phenix_core::Bytes,pub r#mime_type: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural242 { pub r#mime_type: Option<String>,pub r#text: Option<String>,pub r#uri: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural243 { pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural240 { r#Image(Structural241),r#Resource(Structural242),r#Text(Structural243), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural244 { r#Assistant,r#User, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural239 { pub r#content: Vec<Structural240>,pub r#role: Structural244, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural238 { pub r#message: Structural239, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural245 { pub r#title: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct Structural246 { pub r#execution_id: String,pub r#text: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub enum Structural207 { r#Closed,r#Diagnostic(Structural208),r#Execution(Structural211),r#Message(Structural238),r#Renamed(Structural245),r#TextDelta(Structural246), }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSessionUpdate1Type { pub r#sequence: u64,pub r#session_id: String,pub r#update: Structural207, }
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
pub struct Structural247 { pub r#active: bool,pub r#description: String,pub r#id: String,pub r#name: String, }
#[derive(Clone, Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
pub struct PhenixApplicationTypeSkills1Type { pub r#skills: Vec<Structural247>, }
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
pub struct PhenixApplicationCapabilityPermission1Capability; impl PhenixApplicationCapabilityPermission1Capability { pub const ID: &'static str = "phenix.application.capability.permission@1"; }
pub struct PhenixApplicationCapabilityPrompt1Capability; impl PhenixApplicationCapabilityPrompt1Capability { pub const ID: &'static str = "phenix.application.capability.prompt@1"; }
pub struct PhenixApplicationCapabilityRouting1Capability; impl PhenixApplicationCapabilityRouting1Capability { pub const ID: &'static str = "phenix.application.capability.routing@1"; }
pub struct PhenixApplicationCapabilitySessionList1Capability; impl PhenixApplicationCapabilitySessionList1Capability { pub const ID: &'static str = "phenix.application.capability.session-list@1"; }
pub struct PhenixApplicationCapabilitySessionRename1Capability; impl PhenixApplicationCapabilitySessionRename1Capability { pub const ID: &'static str = "phenix.application.capability.session-rename@1"; }
pub struct PhenixApplicationCapabilitySessionResume1Capability; impl PhenixApplicationCapabilitySessionResume1Capability { pub const ID: &'static str = "phenix.application.capability.session-resume@1"; }
pub struct PhenixApplicationCapabilitySessions1Capability; impl PhenixApplicationCapabilitySessions1Capability { pub const ID: &'static str = "phenix.application.capability.sessions@1"; }
pub struct PhenixApplicationCapabilitySkills1Capability; impl PhenixApplicationCapabilitySkills1Capability { pub const ID: &'static str = "phenix.application.capability.skills@1"; }
