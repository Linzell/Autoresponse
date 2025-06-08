# Autoresponse API Documentation for Frontend Developers

This document provides comprehensive documentation for frontend developers integrating with the Autoresponse Tauri backend.

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Service Configuration](#service-configuration)
- [Notifications](#notifications)
- [MCP Server](#mcp-server)
- [Error Handling](#error-handling)
- [Types & Interfaces](#types-and-interfaces)

## Overview

All interactions with the backend are handled through Tauri's invoke system. Import the invoke function:

```typescript
import { invoke } from "@tauri-apps/api/tauri";
```

## Authentication

OAuth2 authentication is handled per service. Tokens are securely stored in the system keychain.

## Service Configuration

### Create Service Config

```typescript
interface CreateServiceConfigRequest {
  name: string;
  serviceType: ServiceType;
  authType: AuthType;
  authConfig: AuthConfig;
  endpoints: ServiceEndpoints;
}

// Usage
const response = await invoke<ServiceConfigResponse>("create_service_config", {
  requestJson: JSON.stringify(request),
});
```

### Get Service Config

```typescript
const config = await invoke<ServiceConfigResponse>("get_service_config", {
  id: "config-id",
});
```

### Get All Service Configs

```typescript
const configs = await invoke<ServiceConfigListResponse>(
  "get_all_service_configs",
);
```

### Update Auth Config

```typescript
interface UpdateServiceAuthRequest {
  authConfig: AuthConfig;
}

await invoke("update_auth_config", {
  id: "config-id",
  requestJson: JSON.stringify(request),
});
```

### Enable/Disable Service

```typescript
await invoke("enable_service", { id: "config-id" });
await invoke("disable_service", { id: "config-id" });
```

### Delete Service Config

```typescript
await invoke("delete_service_config", { id: "config-id" });
```

## Notifications

### Create Notification

```typescript
interface CreateNotificationRequest {
  title: string;
  content: string;
  priority: NotificationPriority;
  metadata: NotificationMetadata;
}

const notification = await invoke<NotificationResponse>("create_notification", {
  requestJson: JSON.stringify(request),
});
```

### Get Notification

```typescript
const notification = await invoke<NotificationResponse>("get_notification", {
  id: "notification-id",
});
```

### Get All Notifications

```typescript
interface NotificationFilterRequest {
  status?: NotificationStatus;
  source?: NotificationSource;
  priority?: NotificationPriority;
  tags?: string[];
  fromDate?: string;
  toDate?: string;
  page?: number;
  perPage?: number;
}

const notifications = await invoke<NotificationListResponse>(
  "get_all_notifications",
  {
    filterJson: JSON.stringify(filter), // Optional
  },
);
```

### Notification Status Management

```typescript
// Mark as read
await invoke("mark_as_read", { id: "notification-id" });

// Mark action required
await invoke("mark_action_required", { id: "notification-id" });

// Mark action taken
await invoke("mark_action_taken", { id: "notification-id" });

// Archive
await invoke("archive_notification", { id: "notification-id" });

// Delete
await invoke("delete_notification", { id: "notification-id" });
```

### Bulk Operations

```typescript
// Mark all as read
await invoke("mark_all_notifications_read");

// Archive all read notifications
await invoke("archive_all_read_notifications");
```

## MCP Server

The Message Control Protocol (MCP) server provides unified access to various services through REST endpoints.

### Health Check

```typescript
GET /health
Response: {
  "success": true,
  "response": "MCP Server is running",
  "error": null
}
```

### Content Analysis

```typescript
POST /api/analyze
Request: {
  "content": string,
  "api_key": string,
  "service_type": "analyze"
}
Response: {
  "success": true,
  "response": {
    "requires_action": boolean,
    "priority_level": "High" | "Medium" | "Low",
    "summary": string,
    "suggested_actions": string[]
  },
  "error": null
}
```

### Response Generation

```typescript
POST /api/generate
Request: {
  "content": string,
  "api_key": string,
  "service_type": "generate"
}
Response: {
  "success": true,
  "response": string,
  "error": null
}
```

### Web Search

```typescript
POST /api/search
Request: {
  "query": string,
  "api_key": string
}
Response: {
  "success": true,
  "response": [
    {
      "title": string,
      "description": string,
      "url": string
    }
  ],
  "error": null
}
```

All MCP server endpoints require authentication via API key and return responses in a standardized format:

```typescript
interface MCPResponse<T> {
  success: boolean;
  response?: T;
  error?: string;
}
```

## Error Handling

For error types and handling, please refer to the [shared types documentation](shared/types.md#error-types).

### Error Handling Example

```typescript
try {
  await invoke("create_service_config", {
    requestJson: JSON.stringify(request),
  });
} catch (error) {
  if (error instanceof Error) {
    // Handle specific error types
    switch (error.constructor.name) {
      case "ValidationError":
        // Handle validation error
        break;
      case "ServiceConfigError":
        // Handle service config error
        break;
      default:
      // Handle unknown error
    }
  }
}
```

// Types & Interfaces

All common types and interfaces are defined in [shared/types.md](shared/types.md). This includes:

- Service Types (ServiceType, AuthType enums and interfaces)
- Authentication Types (OAuth2Config, BasicAuthConfig, etc.)
- Notification Types (NotificationStatus, NotificationPriority, NotificationSource enums)
- Notification Metadata (NotificationMetadata interface)
- Request Types (CreateServiceConfigRequest, UpdateServiceAuthRequest, etc.)
- Response Types (ServiceConfigResponse, NotificationResponse, etc.)
- Error Types

Please refer to the shared types documentation for detailed type definitions.

For additional assistance or to report issues, please refer to the GitHub repository or contact the maintainers.
