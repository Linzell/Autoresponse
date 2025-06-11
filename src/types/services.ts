export enum ServiceType {
  Github = 'Github',
  Gitlab = 'Gitlab',
  Jira = 'Jira',
  Google = 'Google',
  Microsoft = 'Microsoft',
  LinkedIn = 'LinkedIn',
  Custom = 'Custom'
}

export enum AuthType {
  OAuth2 = 'OAuth2',
  BasicAuth = 'BasicAuth',
  ApiKey = 'ApiKey',
  Custom = 'Custom'
}

export interface OAuth2Config {
  clientId: string;
  clientSecret: string;
  redirectUri: string;
  authUrl: string;
  tokenUrl: string;
  scope: string[];
  accessToken?: string;
  refreshToken?: string;
  tokenExpiresAt?: Date;
}

export interface BasicAuthConfig {
  username: string;
  password: string;
}

export interface ApiKeyConfig {
  key: string;
  headerName?: string;
}

export interface CustomAuthConfig {
  authType: string;
  config: any;
}

export type AuthConfig = OAuth2Config | BasicAuthConfig | ApiKeyConfig | CustomAuthConfig;

export interface ServiceMetadata {
  name: string;
  description: string;
  icon: string;
  requiredScopes: string[];
  defaultEndpoints: {
    baseUrl: string;
    endpoints: Record<string, {
      path: string;
      method: string;
    }>;
  };
}

export interface ServiceConnectionState {
  serviceType: ServiceType;
  authType: AuthType;
  isConfigured: boolean;
  isConnected: boolean;
  hasError: boolean;
  errorMessage?: string;
  lastSync?: Date;
  authConfig?: AuthConfig;
}

export const SERVICE_METADATA: Record<ServiceType, ServiceMetadata> = {
  Github: {
    name: 'GitHub',
    description: 'Connect to GitHub for repository and issue notifications',
    icon: 'github',
    requiredScopes: ['repo', 'user', 'notifications'],
    defaultEndpoints: {
      baseUrl: 'https://api.github.com',
      endpoints: {
        notifications: {
          path: '/notifications',
          method: 'GET'
        },
        issues: {
          path: '/issues',
          method: 'GET'
        }
      }
    }
  },
  Gitlab: {
    name: 'GitLab',
    description: 'Connect to GitLab for merge requests and issue notifications',
    icon: 'gitlab',
    requiredScopes: ['api', 'read_user'],
    defaultEndpoints: {
      baseUrl: 'https://gitlab.com/api/v4',
      endpoints: {
        notifications: {
          path: '/todos',
          method: 'GET'
        },
        mergeRequests: {
          path: '/merge_requests',
          method: 'GET'
        }
      }
    }
  },
  Google: {
    name: 'Google',
    description: 'Connect to Google services including Gmail and Calendar',
    icon: 'google',
    requiredScopes: ['email', 'profile', 'https://www.googleapis.com/auth/gmail.modify'],
    defaultEndpoints: {
      baseUrl: 'https://www.googleapis.com',
      endpoints: {
        messages: {
          path: '/gmail/v1/users/me/messages',
          method: 'GET'
        },
        profile: {
          path: '/oauth2/v2/userinfo',
          method: 'GET'
        }
      }
    }
  },
  Microsoft: {
    name: 'Microsoft',
    description: 'Connect to Microsoft services including Outlook and Teams',
    icon: 'microsoft',
    requiredScopes: ['offline_access', 'User.Read', 'Mail.ReadWrite'],
    defaultEndpoints: {
      baseUrl: 'https://graph.microsoft.com/v1.0',
      endpoints: {
        messages: {
          path: '/me/messages',
          method: 'GET'
        },
        profile: {
          path: '/me',
          method: 'GET'
        }
      }
    }
  },
  Jira: {
    name: 'Jira',
    description: 'Connect to Jira for issue and project notifications',
    icon: 'jira',
    requiredScopes: ['read:jira-work', 'read:jira-user'],
    defaultEndpoints: {
      baseUrl: 'https://api.atlassian.com/ex/jira',
      endpoints: {
        issues: {
          path: '/rest/api/3/search',
          method: 'GET'
        },
        projects: {
          path: '/rest/api/3/project',
          method: 'GET'
        }
      }
    }
  },
  LinkedIn: {
    name: 'LinkedIn',
    description: 'Connect to LinkedIn for professional network updates',
    icon: 'linkedin',
    requiredScopes: ['r_liteprofile', 'r_emailaddress', 'w_member_social'],
    defaultEndpoints: {
      baseUrl: 'https://api.linkedin.com/v2',
      endpoints: {
        profile: {
          path: '/me',
          method: 'GET'
        },
        network: {
          path: '/connections',
          method: 'GET'
        }
      }
    }
  },
  Custom: {
    name: 'Custom Service',
    description: 'Configure a custom service integration',
    icon: 'custom',
    requiredScopes: [],
    defaultEndpoints: {
      baseUrl: '',
      endpoints: {}
    }
  }
};
