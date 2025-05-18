import React, { useState, useEffect } from 'react';
import {
  Box,
  Button,
  Card,
  CardContent,
  TextField,
  Typography,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  Alert,
  CircularProgress,
} from '@mui/material';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

interface OAuthCredentials {
  clientId: string;
  clientSecret: string;
  serviceType: ServiceType;
}

enum ServiceType {
  Github = 'Github',
  Google = 'Google',
  Microsoft = 'Microsoft',
  Gitlab = 'Gitlab',
  LinkedIn = 'LinkedIn',
}

interface ServiceEndpoints {
  baseUrl: string;
  endpoints: Record<string, unknown>;
}

interface ServiceConfig {
  id: string;
  name: string;
  serviceType: ServiceType;
  authConfig: OAuthCredentials;
  endpoints: ServiceEndpoints;
  enabled: boolean;
}

export const OAuthConfig: React.FC = () => {
  const [configs, setConfigs] = useState<ServiceConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [selectedService, setSelectedService] = useState<ServiceType>(ServiceType.Github);
  const [credentials, setCredentials] = useState<OAuthCredentials>({
    clientId: '',
    clientSecret: '',
    serviceType: ServiceType.Github,
  });

  useEffect(() => {
    loadConfigs();
    setupOAuthListener();
  }, []);

  const setupOAuthListener = async () => {
    await listen('oauth-callback', (event: any) => {
      const { success, configId, error } = event.payload;
      if (success) {
        setSuccess(`OAuth configuration completed successfully!`);
        loadConfigs();
      } else {
        setError(`OAuth configuration failed: ${error}`);
      }
      setLoading(false);
    });
  };

  const loadConfigs = async () => {
    try {
      const configs = await invoke<ServiceConfig[]>('get_service_configs');
      setConfigs(configs);
      setLoading(false);
    } catch (err) {
      setError(`Failed to load configurations: ${err}`);
      setLoading(false);
    }
  };

  const handleServiceChange = (event: any) => {
    const serviceType = event.target.value as ServiceType;
    setSelectedService(serviceType);
    setCredentials({ ...credentials, serviceType });
  };

  const handleInputChange = (field: keyof OAuthCredentials) => (event: React.ChangeEvent<HTMLInputElement>) => {
    setCredentials({ ...credentials, [field]: event.target.value });
  };

  const handleSave = async () => {
    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      const savedConfig = await invoke<ServiceConfig>('save_oauth_config', {
        credentials: {
          ...credentials,
          serviceType: selectedService,
        },
      });

      setSuccess('Configuration saved successfully!');
      await loadConfigs();
    } catch (err) {
      setError(`Failed to save configuration: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleStartAuth = async (config: ServiceConfig) => {
    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      const authUrl = await invoke<string>('start_oauth_flow', {
        serviceType: config.serviceType,
      });
      window.open(authUrl, '_blank');
    } catch (err) {
      setError(`Failed to start OAuth flow: ${err}`);
      setLoading(false);
    }
  };

  const handleDelete = async (configId: string) => {
    if (!confirm('Are you sure you want to delete this configuration?')) {
      return;
    }

    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      await invoke('delete_oauth_service_config', { configId });
      setSuccess('Configuration deleted successfully!');
      await loadConfigs();
    } catch (err) {
      setError(`Failed to delete configuration: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box sx={{ maxWidth: 800, margin: '0 auto', padding: 2 }}>
      <Typography variant="h5" gutterBottom>
        OAuth Service Configuration
      </Typography>

      {error && (
        <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      {success && (
        <Alert severity="success" sx={{ mb: 2 }} onClose={() => setSuccess(null)}>
          {success}
        </Alert>
      )}

      <Card sx={{ mb: 4 }}>
        <CardContent>
          <FormControl fullWidth sx={{ mb: 2 }}>
            <InputLabel>Service Type</InputLabel>
            <Select value={selectedService} onChange={handleServiceChange}>
              {Object.values(ServiceType).map((type) => (
                <MenuItem key={type} value={type}>
                  {type}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          <TextField
            fullWidth
            label="Client ID"
            value={credentials.clientId}
            onChange={handleInputChange('clientId')}
            sx={{ mb: 2 }}
          />

          <TextField
            fullWidth
            label="Client Secret"
            type="password"
            value={credentials.clientSecret}
            onChange={handleInputChange('clientSecret')}
            sx={{ mb: 2 }}
          />

          <Button
            variant="contained"
            color="primary"
            onClick={handleSave}
            disabled={loading || !credentials.clientId || !credentials.clientSecret}
          >
            {loading ? <CircularProgress size={24} /> : 'Save Configuration'}
          </Button>
        </CardContent>
      </Card>

      <Typography variant="h6" gutterBottom>
        Configured Services
      </Typography>

      {configs.map((config) => (
        <Card key={config.id} sx={{ mb: 2 }}>
          <CardContent>
            <Typography variant="h6" gutterBottom>
              {config.name}
            </Typography>
            <Typography color="textSecondary" gutterBottom>
              Service Type: {config.serviceType}
            </Typography>
            <Typography color="textSecondary" gutterBottom>
              Status: {config.enabled ? 'Enabled' : 'Disabled'}
            </Typography>
            <Box sx={{ mt: 2 }}>
              <Button
                variant="contained"
                color="primary"
                onClick={() => handleStartAuth(config)}
                sx={{ mr: 1 }}
              >
                Start Authentication
              </Button>
              <Button
                variant="outlined"
                color="error"
                onClick={() => handleDelete(config.id)}
              >
                Delete
              </Button>
            </Box>
          </CardContent>
        </Card>
      ))}
    </Box>
  );
};

export default OAuthConfig;
