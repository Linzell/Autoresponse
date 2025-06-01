use crate::domain::error::DomainError;
use async_trait::async_trait;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

pub trait CachedEntity: Clone + Send + Sync + 'static {
    fn get_id(&self) -> Uuid;
}

#[async_trait]
pub trait Repository<T: CachedEntity> {
    async fn save(&self, entity: &mut T) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<T>, DomainError>;
    async fn find_all(&self) -> Result<Vec<T>, DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}

#[derive(Debug)]
pub struct CachedRepository<T, R>
where
    T: CachedEntity,
    R: Repository<T>,
{
    pub repository: Arc<R>,
    cache: Cache<Uuid, T>,
}

impl<T, R> CachedRepository<T, R>
where
    T: CachedEntity,
    R: Repository<T>,
{
    pub fn new(repository: Arc<R>, max_capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();

        Self { repository, cache }
    }

    pub async fn invalidate(&self, id: Uuid) {
        self.cache.invalidate(&id).await;
    }

    pub async fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }
}

#[async_trait]
impl<T, R> Repository<T> for CachedRepository<T, R>
where
    T: CachedEntity,
    R: Repository<T> + Send + Sync,
{
    async fn save(&self, entity: &mut T) -> Result<(), DomainError> {
        let result = self.repository.save(entity).await;
        if result.is_ok() {
            self.cache.insert(entity.get_id(), entity.clone()).await;
        }
        result
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<T>, DomainError> {
        if let Some(cached) = self.cache.get(&id).await {
            return Ok(Some(cached));
        }

        let result = self.repository.find_by_id(id).await?;
        if let Some(entity) = result.clone() {
            self.cache.insert(id, entity).await;
        }
        Ok(result)
    }

    async fn find_all(&self) -> Result<Vec<T>, DomainError> {
        self.repository.find_all().await
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let result = self.repository.delete(id).await;
        if result.is_ok() {
            self.cache.invalidate(&id).await;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use mockall::predicate::*;
    use std::time::Duration;

    #[derive(Clone)]
    struct TestEntity {
        id: Uuid,
        value: String,
    }

    impl CachedEntity for TestEntity {
        fn get_id(&self) -> Uuid {
            self.id
        }
    }

    mock! {
        TestRepository {}
        #[async_trait]
        impl Repository<TestEntity> for TestRepository {
            async fn save(&self, entity: &mut TestEntity) -> Result<(), DomainError>;
            async fn find_by_id(&self, id: Uuid) -> Result<Option<TestEntity>, DomainError>;
            async fn find_all(&self) -> Result<Vec<TestEntity>, DomainError>;
            async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
        }
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let mut mock_repo = MockTestRepository::new();
        let id = Uuid::new_v4();
        let entity = TestEntity {
            id,
            value: "test".to_string(),
        };

        // First call should hit the repository
        mock_repo
            .expect_find_by_id()
            .with(eq(id))
            .times(1)
            .returning(move |_| Ok(Some(entity.clone())));

        let cached_repo = CachedRepository::new(Arc::new(mock_repo), 100, Duration::from_secs(60));

        // First call - should hit repository
        let result1 = cached_repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(result1.value, "test");

        // Second call - should hit cache
        let result2 = cached_repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(result2.value, "test");
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let mut mock_repo = MockTestRepository::new();
        let id = Uuid::new_v4();
        let mut entity = TestEntity {
            id,
            value: "test".to_string(),
        };

        mock_repo.expect_save().returning(|_| Ok(()));

        mock_repo.expect_delete().returning(|_| Ok(()));

        let cached_repo = CachedRepository::new(Arc::new(mock_repo), 100, Duration::from_secs(60));

        // Save entity
        cached_repo.save(&mut entity).await.unwrap();

        // Delete entity should invalidate cache
        cached_repo.delete(id).await.unwrap();

        // Cache should be empty
        assert!(cached_repo.cache.get(&id).await.is_none());
    }
}
