use crate::{
    ApiTokenScheme, ApiTokenSource, Auth, AuthKind, EnvironmentVariable,
    EnvironmentVariableParseError, HeaderName, HeaderNameParseError, Token, TokenParseError,
};

pub use crate::{
    ApiTokenScheme as ApiTokenMethod, ApiTokenSource as ApiToken, Auth as Credential,
    AuthDescriptor as CredentialDescriptor,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Definition {
    pub api_token: Option<ApiTokenMethod>,
    pub oauth: Option<OAuthMethod>,
}

impl Definition {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            api_token: None,
            oauth: None,
        }
    }

    #[must_use]
    pub const fn api_token(api_token: ApiTokenMethod) -> Self {
        Self {
            api_token: Some(api_token),
            oauth: None,
        }
    }

    #[must_use]
    pub const fn oauth(oauth: OAuthMethod) -> Self {
        Self {
            api_token: None,
            oauth: Some(oauth),
        }
    }

    #[must_use]
    pub fn with_api_token(mut self, api_token: ApiTokenMethod) -> Self {
        self.api_token = Some(api_token);
        self
    }

    #[must_use]
    pub fn with_oauth(mut self, oauth: OAuthMethod) -> Self {
        self.oauth = Some(oauth);
        self
    }

    pub(crate) fn kinds(&self) -> Vec<AuthKind> {
        let mut kinds = Vec::with_capacity(2);
        if self.api_token.is_some() {
            kinds.push(AuthKind::ApiToken);
        }
        if self.oauth.is_some() {
            kinds.push(AuthKind::OAuth);
        }
        kinds
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.api_token.is_none() && self.oauth.is_none()
    }
}

impl From<ApiTokenMethod> for Definition {
    fn from(value: ApiTokenMethod) -> Self {
        Self::api_token(value)
    }
}

impl From<OAuthMethod> for Definition {
    fn from(value: OAuthMethod) -> Self {
        Self::oauth(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OAuthMethod {
    Bearer,
}

impl OAuthMethod {
    #[must_use]
    pub const fn bearer() -> Self {
        Self::Bearer
    }
}

impl ApiTokenScheme {
    #[must_use]
    pub const fn bearer() -> Self {
        Self::Bearer
    }

    pub fn header(name: impl Into<String>) -> Result<Self, HeaderNameParseError> {
        Ok(Self::Header {
            name: HeaderName::parse(name)?,
        })
    }
}

impl ApiTokenSource {
    pub fn literal(token: impl Into<String>) -> Result<Self, TokenParseError> {
        Ok(Self::Literal {
            token: Token::parse(token)?,
        })
    }

    pub fn env(variable: impl Into<String>) -> Result<Self, EnvironmentVariableParseError> {
        Ok(Self::Environment {
            variable: EnvironmentVariable::parse(variable)?,
        })
    }
}

impl Auth {
    #[must_use]
    pub fn api_token(source: ApiTokenSource) -> Self {
        Self::ApiToken { source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_composes_auth_methods_without_duplicate_state() {
        let definition =
            Definition::api_token(ApiTokenMethod::bearer()).with_oauth(OAuthMethod::bearer());

        assert_eq!(
            definition.kinds(),
            vec![AuthKind::ApiToken, AuthKind::OAuth]
        );
    }

    #[test]
    fn api_token_credentials_parse_at_construction() {
        assert!(matches!(
            ApiToken::env("OPENAI_API_KEY").unwrap(),
            ApiTokenSource::Environment { .. }
        ));
        assert!(ApiToken::env("not-valid").is_err());
        assert!(matches!(
            ApiToken::literal("secret").unwrap(),
            ApiTokenSource::Literal { .. }
        ));
    }
}
