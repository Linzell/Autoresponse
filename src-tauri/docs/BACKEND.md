# Autoresponse Backend Documentation

## Table of Contents
1. [Architecture Overview](#architecture-overview)
2. [Domain Model](#domain-model)
3. [Backend Commands API](#backend-commands-api)
4. [Error Handling](#error-handling)
5. [Types and Interfaces](#types-and-interfaces)
6. [Integration Examples](#integration-examples)
7. [Development Guide](#development-guide)

> **Note**: For detailed implementation patterns and guidelines, please refer to [DEVELOPMENT.md](DEVELOPMENT.md)

## Architecture Overview

The backend follows Domain-Driven Design (DDD) principles with a clean architecture approach:

```
src-tauri/
├── application/     # Use cases and application services
├── domain/         # Core business logic and interfaces
├── infrastructure/ # External implementations (DB, APIs)
└── presentation/   # Controllers and DTOs
```

For detailed implementation patterns and best practices for each layer, see the [Development Guide](DEVELOPMENT.md#implementation-patterns).

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
// See shared/types.md for CreateNotificationRequest interface

await invoke('create_notification', {
  requestJson: JSON.stringify(request)
});
```

#### Get Notifications
```typescript
// Get by ID
await invoke('get_notification', { id: string });

// Get All with Filtering
// See shared/types.md for NotificationFilterRequest interface

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

For detailed error type definitions, please refer to the [shared types documentation](shared/types.md#error-types).

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

### Types and Interfaces

All common types and interfaces are defined in the [shared types documentation](shared/types.md). This includes:

- Service Types and Configurations
- Notification Types and Statuses
- Request/Response Types
- Error Types and Handling

Please refer to the shared types documentation for detailed type definitions.

## Integration Examples

> **Note**: For implementation patterns and best practices when adding new features, see the [Development Guide](DEVELOPMENT.md#adding-new-features).

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

// See DEVELOPMENT.md for comprehensive implementation patterns and error handling
```

### Notification Management Flow
```typescript
// Example notification creation
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

// For more examples and best practices, refer to DEVELOPMENT.md
```

## Development Guide

For detailed information about:
- Implementation patterns
- Testing guidelines
- Error handling
- Best practices
- Adding new features

Please refer to the [Development Guide](DEVELOPMENT.md).