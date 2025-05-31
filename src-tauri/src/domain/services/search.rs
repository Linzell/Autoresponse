use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub description: String,
    pub url: String,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SearchService: Send + Sync + std::fmt::Debug {
    async fn search(&self, query: &str) -> DomainResult<Vec<SearchResult>>;
}

#[derive(Debug, Clone)]
pub struct BraveSearchService {
    api_key: String,
    base_url: String,
}

impl BraveSearchService {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.search.brave.com/res/v1/web/search".to_string(),
        }
    }
}

#[async_trait]
impl SearchService for BraveSearchService {
    async fn search(&self, query: &str) -> DomainResult<Vec<SearchResult>> {
        let client = reqwest::Client::new();

        let response = client
            .get(&self.base_url)
            .query(&[("q", query)])
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalServiceError(format!("Search request failed: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(DomainError::ExternalServiceError(format!(
                "Search API returned error status: {}",
                response.status()
            )));
        }

        let search_response = response.json::<serde_json::Value>().await.map_err(|e| {
            DomainError::ExternalServiceError(format!("Failed to parse response: {}", e))
        })?;

        let results = search_response["web"]["results"]
            .as_array()
            .ok_or_else(|| {
                DomainError::ExternalServiceError("Invalid response format".to_string())
            })?
            .iter()
            .map(|result| {
                Ok(SearchResult {
                    title: result["title"]
                        .as_str()
                        .ok_or_else(|| {
                            DomainError::ExternalServiceError("Missing title".to_string())
                        })?
                        .to_string(),
                    description: result["description"]
                        .as_str()
                        .ok_or_else(|| {
                            DomainError::ExternalServiceError("Missing description".to_string())
                        })?
                        .to_string(),
                    url: result["url"]
                        .as_str()
                        .ok_or_else(|| {
                            DomainError::ExternalServiceError("Missing URL".to_string())
                        })?
                        .to_string(),
                })
            })
            .collect::<DomainResult<Vec<_>>>()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use mockall::predicate::*;

    mock! {
        #[derive(Debug)]
        SearchServiceMock {}
        #[async_trait]
        impl SearchService for SearchServiceMock {
            async fn search(&self, query: &str) -> DomainResult<Vec<SearchResult>>;
        }
    }

    #[tokio::test]
    async fn test_mock_search_service() {
        let mut mock = MockSearchServiceMock::new();

        mock.expect_search()
            .with(eq("test query"))
            .times(1)
            .returning(|_| {
                Ok(vec![SearchResult {
                    title: "Test Result".to_string(),
                    description: "Test Description".to_string(),
                    url: "http://test.com".to_string(),
                }])
            });

        let result = mock.search("test query").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Test Result");
    }
}
