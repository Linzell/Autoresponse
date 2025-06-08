# Shared Types and Interfaces

## Service Types

```typescript
enum ServiceType {
  Email = "Email",
  Github = "Github",
  Gitlab = "Gitlab",
  Jira = "Jira",
  Google = "Google",
  Microsoft = "Microsoft",
  LinkedIn = "LinkedIn",
  Custom = string, // Custom service type with string identifier
}

interface ServiceConfig {
  id: string;
  name: string;
  serviceType: ServiceType;
  authType: AuthType;
  authConfig: AuthConfig;
  endpoints: ServiceEndpoints;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
  lastSync?: string;
  metadata: Record<string, unknown>;
}

enum AuthType {
  OAuth2 = "OAuth2",
  BasicAuth = "BasicAuth",
  ApiKey = "ApiKey",
  Custom = string, // Custom auth type with string identifier
}

interface OAuth2Config {
  clientId: string;
  clientSecret: string;
  redirectUri: string;
  authUrl: string;
  tokenUrl: string;
  scope: string[];
  accessToken?: string;
  refreshToken?: string;
  tokenExpiresAt?: string;
}

interface BasicAuthConfig {
  username: string;
  password: string;
}

interface ApiKeyConfig {
  key: string;
  headerName?: string;
}

interface CustomAuthConfig {
  authType: string;
  config: Record<string, unknown>;
}

type AuthConfig = {
  OAuth2: OAuth2Config;
  BasicAuth: BasicAuthConfig;
  ApiKey: ApiKeyConfig;
  Custom: CustomAuthConfig;
};

interface ServiceEndpoints {
  baseUrl: string;
  endpoints: Record<string, unknown>;
}
```

## Notification Types

```typescript
enum NotificationStatus {
  New = "New",
  Read = "Read",
  Archived = "Archived",
  ActionRequired = "ActionRequired",
  ActionTaken = "ActionTaken",
  Deleted = "Deleted",
}

enum NotificationPriority {
  Low = "Low",
  Medium = "Medium",
  High = "High",
  Critical = "Critical",
}

enum NotificationSource {
  Email = "Email",
  Github = "Github",
  Gitlab = "Gitlab",
  Jira = "Jira",
  Microsoft = "Microsoft",
  Google = "Google",
  LinkedIn = "LinkedIn",
  Custom = string, // Custom source with string identifier
}

interface Notification {
  id: string;
  title: string;
  content: string;
  priority: NotificationPriority;
  status: NotificationStatus;
  metadata: NotificationMetadata;
  createdAt: string;
  updatedAt: string;
  readAt?: string;
  actionTakenAt?: string;
}

interface NotificationMetadata {
  source: NotificationSource;
  externalId?: string;
  url?: string;
  tags: string[];
  customData?: Record<string, unknown>;
}
```

## Error Types

```typescript
interface ValidationError {
  message: string;
  field?: string;
}

interface ServiceConfigError {
  message: string;
  code: string;
}

interface NotificationError {
  message: string;
  code: string;
}
```

## Request Types

```typescript
interface CreateServiceConfigRequest {
  name: string;
  serviceType: ServiceType;
  authType: AuthType;
  authConfig: AuthConfig;
  endpoints: ServiceEndpoints;
}

interface UpdateServiceAuthRequest {
  clientId?: string;
  clientSecret?: string;
  accessToken?: string;
  refreshToken?: string;
  tokenExpiry?: string;
}

interface CreateNotificationRequest {
  title: string;
  content: string;
  priority: NotificationPriority;
  metadata: NotificationMetadata;
}

interface NotificationFilterRequest {
  status?: NotificationStatus;
  source?: string;
  priority?: NotificationPriority;
  tags?: string[];
  fromDate?: string;
  toDate?: string;
  page?: number;
  perPage?: number;
}
```

## Response Types

```typescript
interface ServiceConfigResponse {
  id: string;
  name: string;
  type: ServiceType;
  enabled: boolean;
  authConfig: {
    clientId: string;
    redirectUri?: string;
    scopes?: string[];
  };
  createdAt: string;
  updatedAt: string;
}

interface NotificationResponse {
  id: string;
  title: string;
  content: string;
  source: string;
  status: NotificationStatus;
  priority: NotificationPriority;
  tags: string[];
  createdAt: string;
  updatedAt: string;
}

interface ServiceConfigListResponse {
  configs: ServiceConfigResponse[];
  total: number;
}

interface NotificationListResponse {
  notifications: NotificationResponse[];
  total: number;
  page?: number;
  perPage?: number;
}
```

## Usage Notes

1. All date strings follow ISO 8601 format
2. All IDs are UUID v4 strings
3. Metadata objects should be serializable to JSON
4. Enums are string literal types in TypeScript
5. All response types include timestamps

For additional type information or clarification, please refer to the API and Backend documentation.
