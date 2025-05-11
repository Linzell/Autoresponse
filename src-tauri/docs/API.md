# Autoresponse API Documentation for Frontend Developers

This document provides comprehensive documentation for frontend developers integrating with the Autoresponse Tauri backend.

## Table of Contents
- [Overview](#overview)
- [Authentication](#authentication)
- [Service Configuration](#service-configuration)
- [Notifications](#notifications)
- [Error Handling](#error-handling)
- [Types & Interfaces](#types-and-interfaces)

## Overview

All interactions with the backend are handled through Tauri's invoke system. Import the invoke function:

```typescript
import { invoke } from '@tauri-apps/api/tauri';
```

## Authentication

OAuth2 authentication is handled per service. Tokens are securely stored in the system keychain.

## Service Configuration

### Create Service Config
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
const response = await invoke<ServiceConfigResponse>('create_service_config', {
  requestJson: JSON.stringify(request)
});
```

### Get Service Config
```typescript
const config = await invoke<ServiceConfigResponse>('get_service_config', {
  id: 'config-id'
});
```

### Get All Service Configs
```typescript
const configs = await invoke<ServiceConfigListResponse>('get_all_service_configs');
```

### Update Auth Config
```typescript
interface UpdateServiceAuthRequest {
  clientId?: string;
  clientSecret?: string;
  redirectUri?: string;
  scopes?: string[];
}

await invoke('update_auth_config', {
  id: 'config-id',
  requestJson: JSON.stringify(request)
});
```

### Enable/Disable Service
```typescript
await invoke('enable_service', { id: 'config-id' });
await invoke('disable_service', { id: 'config-id' });
```

### Delete Service Config
```typescript
await invoke('delete_service_config', { id: 'config-id' });
```

## Notifications

### Create Notification
```typescript
interface CreateNotificationRequest {
  title: string;
  content: string;
  source: string;
  priority?: NotificationPriority;
  tags?: string[];
}

const notification = await invoke<NotificationResponse>('create_notification', {
  requestJson: JSON.stringify(request)
});
```

### Get Notification
```typescript
const notification = await invoke<NotificationResponse>('get_notification', {
  id: 'notification-id'
});
```

### Get All Notifications
```typescript
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

const notifications = await invoke<NotificationListResponse>('get_all_notifications', {
  filterJson: JSON.stringify(filter) // Optional
});
```

### Notification Status Management
```typescript
// Mark as read
await invoke('mark_as_read', { id: 'notification-id' });

// Mark action required
await invoke('mark_action_required', { id: 'notification-id' });

// Mark action taken
await invoke('mark_action_taken', { id: 'notification-id' });

// Archive
await invoke('archive_notification', { id: 'notification-id' });

// Delete
await invoke('delete_notification', { id: 'notification-id' });
```

### Bulk Operations
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
    // Handle specific error types
    switch (error.constructor.name) {
      case 'ValidationError':
        // Handle validation error
        break;
      case 'ServiceConfigError':
        // Handle service config error
        break;
      default:
        // Handle unknown error
    }
  }
}
```

## Types and Interfaces

### Service Types
```typescript
enum ServiceType {
  EMAIL = 'EMAIL',
  GIT_SERVICE = 'GIT_SERVICE',
  MICROSOFT = 'MICROSOFT',
  GOOGLE = 'GOOGLE',
  JIRA = 'JIRA',
  LINKEDIN = 'LINKEDIN',
  CUSTOM = 'CUSTOM'
}
```

### Notification Types
```typescript
enum NotificationStatus {
  UNREAD = 'UNREAD',
  READ = 'READ',
  ACTION_REQUIRED = 'ACTION_REQUIRED',
  ACTION_TAKEN = 'ACTION_TAKEN',
  ARCHIVED = 'ARCHIVED'
}

enum NotificationPriority {
  LOW = 'LOW',
  MEDIUM = 'MEDIUM',
  HIGH = 'HIGH',
  URGENT = 'URGENT'
}
```

### Response Types
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

For additional assistance or to report issues, please refer to the GitHub repository or contact the maintainers.