# Autoresponse Backend Documentation

## Table of Contents
1. [Architecture Overview](#architecture-overview)
2. [Domain Model](#domain-model)
3. [Backend Commands API](#backend-commands-api)
4. [Error Handling](#error-handling)
5. [Types and Interfaces](#types-and-interfaces)
6. [Integration Examples](#integration-examples)

## Architecture Overview

The backend follows Domain-Driven Design (DDD) principles with a clean architecture approach:

```
src-tauri/
├── application/     # Use cases and application services
├── domain/         # Core business logic and interfaces
├── infrastructure/ # External implementations (DB, APIs)
└── presentation/   # Controllers and DTOs
```

### Key Components
- **Controllers**: Handle command/query routing and validation
- **Use Cases**: Implement business logic workflows
- **Services**: Domain-specific business operations
- **Repositories**: Data persistence abstraction
- **Entities**: Core domain models

## Domain Model

### Service Configuration
- Represents external service integrations
- Manages authentication and connection settings
- Handles service lifecycle (enable/disable)

### Notification
- Core entity for all system notifications
- Manages notification lifecycle and state
- Supports prioritization and categorization

## Backend Commands API

### Service Configuration Commands

#### Create Service Config
```typescript
interface CreateServiceConfigRequest {
  name: string;
  type: ServiceType;
  authConfig: {
    clientId: string;
    clientSecret: string;
    redirectUri?: string;
    scopes?: string[];
  };
}

// Usage
await invoke('create_service_config', {
  requestJson: JSON.stringify(request)
});
```

#### Get Service Config
```typescript
// By ID
await invoke('get_service_config', { id: string });

// Get All
await invoke('get_all_service_configs');
```

#### Update Auth Config
```typescript
interface UpdateServiceAuthRequest {
  clientId?: string;
  clientSecret?: string;
  accessToken?: string;
  refreshToken?: string;
  tokenExpiry?: string;
}

await invoke('update_auth_config', {
  id: string,
  requestJson: JSON.stringify(request)
});
```

#### Service Status Management
```typescript
// Enable service
await invoke('enable_service', { id: string });

// Disable service
await invoke('disable_service', { id: string });

// Delete service
await invoke('delete_service_config', { id: string });
```

### Notification Commands

#### Create Notification
```typescript
interface CreateNotificationRequest {
  title: string;
  content: string;
  source: string;
  priority?: 'LOW' | 'MEDIUM' | 'HIGH';
  tags?: string[];
  metadata?: Record<string, unknown>;
}

await invoke('create_notification', {
  requestJson: JSON.stringify(request)
});
```

#### Get Notifications
```typescript
// Get by ID
await invoke('get_notification', { id: string });

// Get All with Filtering
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

await invoke('get_all_notifications', {
  filterJson: JSON.stringify(filter)
});
```

#### Notification State Management
```typescript
// Mark as read
await invoke('mark_as_read', { id: string });

// Mark action required
await invoke('mark_action_required', { id: string });

// Mark action taken
await invoke('mark_action_taken', { id: string });

// Archive
await invoke('archive_notification', { id: string });

// Delete
await invoke('delete_notification', { id: string });
```

#### Bulk Operations
```typescript
// Mark all as read
await invoke('mark_all_notifications_read');

// Archive all read notifications
await invoke('archive_all_read_notifications');
```

## Error Handling

### Error Types

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

### Error Handling Example
```typescript
try {
  await invoke('create_service_config', {
    requestJson: JSON.stringify(request)
  });
} catch (error) {
  if (error instanceof Error) {
    // Handle validation errors
    if ('field' in error) {
      console.error(`Validation error in field ${error.field}: ${error.message}`);
    }
    // Handle service errors
    else if ('code' in error) {
      console.error(`Service error ${error.code}: ${error.message}`);
    }
    // Handle unknown errors
    else {
      console.error('Unknown error:', error.message);
    }
  }
}
```

## Types and Interfaces

### Service Types
```typescript
type ServiceType = 
  | 'EMAIL'
  | 'GITHUB'
  | 'GITLAB'
  | 'MICROSOFT'
  | 'GOOGLE'
  | 'JIRA'
  | 'LINKEDIN'
  | 'CUSTOM';

interface ServiceConfig {
  id: string;
  name: string;
  type: ServiceType;
  enabled: boolean;
  authConfig: AuthConfig;
  createdAt: string;
  updatedAt: string;
}
```

### Notification Types
```typescript
type NotificationStatus = 
  | 'UNREAD'
  | 'READ'
  | 'ACTION_REQUIRED'
  | 'ACTION_TAKEN'
  | 'ARCHIVED';

type NotificationPriority = 'LOW' | 'MEDIUM' | 'HIGH';

interface Notification {
  id: string;
  title: string;
  content: string;
  source: string;
  status: NotificationStatus;
  priority: NotificationPriority;
  tags: string[];
  metadata: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}
```

## Integration Examples

### Service Configuration Flow
```typescript
// 1. Create service config
const createServiceConfig = async () => {
  const config: CreateServiceConfigRequest = {
    name: 'Github',
    type: 'GITHUB',
    authConfig: {
      clientId: 'your-client-id',
      clientSecret: 'your-client-secret',
      redirectUri: 'http://localhost:3000/callback',
      scopes: ['repo', 'user']
    }
  };

  try {
    const response = await invoke('create_service_config', {
      requestJson: JSON.stringify(config)
    });
    return response;
  } catch (error) {
    console.error('Failed to create service config:', error);
    throw error;
  }
};

// 2. Update auth after OAuth flow
const updateAuthToken = async (serviceId: string, tokens: any) => {
  const authUpdate: UpdateServiceAuthRequest = {
    accessToken: tokens.access_token,
    refreshToken: tokens.refresh_token,
    tokenExpiry: tokens.expires_at
  };

  try {
    await invoke('update_auth_config', {
      id: serviceId,
      requestJson: JSON.stringify(authUpdate)
    });
  } catch (error) {
    console.error('Failed to update auth config:', error);
    throw error;
  }
};
```

### Notification Management Flow
```typescript
// 1. Create notification
const createNotification = async () => {
  const notification: CreateNotificationRequest = {
    title: 'New Pull Request',
    content: 'A new pull request requires your review',
    source: 'github',
    priority: 'HIGH',
    tags: ['review', 'pull-request'],
    metadata: {
      repositoryId: '123',
      prNumber: '456'
    }
  };

  try {
    const response = await invoke('create_notification', {
      requestJson: JSON.stringify(notification)
    });
    return response;
  } catch (error) {
    console.error('Failed to create notification:', error);
    throw error;
  }
};

// 2. Get filtered notifications
const getFilteredNotifications = async () => {
  const filter: NotificationFilterRequest = {
    status: 'UNREAD',
    priority: 'HIGH',
    tags: ['review'],
    fromDate: '2024-01-01',
    page: 1,
    perPage: 20
  };

  try {
    const response = await invoke('get_all_notifications', {
      filterJson: JSON.stringify(filter)
    });
    return response;
  } catch (error) {
    console.error('Failed to get notifications:', error);
    throw error;
  }
};
```