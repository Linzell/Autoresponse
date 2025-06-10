import React, { useState, useEffect } from 'react';
import {
  Box,
  Typography,
  Button,
  Grid,
  Card,
  CardContent,
  CardActions,
  IconButton,
  CircularProgress,
  Alert,
  Tooltip,
} from '@mui/material';
import { useSetupWizard } from '../SetupWizardProvider';
import { useDeepLinkAuth } from '../hooks/useDeepLinkAuth';
import { SERVICE_METADATA, ServiceType, ServiceConnectionState, AuthType } from '../../../types/services';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import RefreshIcon from '@mui/icons-material/Refresh';
import { open } from '@tauri-apps/plugin-shell';
import { invoke } from '@tauri-apps/api/core';

const ServicesStep: React.FC = () => {
  const { nextStep, markStepComplete, addServiceConnection, configuredServices } = useSetupWizard();
  const isHandlingAuth = useDeepLinkAuth();
  const [loading, setLoading] = useState<Record<ServiceType, boolean>>({} as Record<ServiceType, boolean>);
  const [errors, setErrors] = useState<Record<ServiceType, string>>({} as Record<ServiceType, string>);
  const [connections, setConnections] = useState<Record<ServiceType, ServiceConnectionState>>(
    {} as Record<ServiceType, ServiceConnectionState>
  );

  const handleConnectService = async (serviceType: ServiceType) => {
    setLoading(prev => ({ ...prev, [serviceType]: true }));
    setErrors(prev => ({ ...prev, [serviceType]: '' }));

    try {
      // Start OAuth flow
      const authUrl = await invoke('start_oauth_flow', { serviceType });
      console.log('Auth URL:', authUrl);

      // Open auth URL in default system browser
      try {
        await open(authUrl as string);
      } catch (error) {
        throw new Error('Failed to open authentication browser window: ' + error);
      }

      // Handle the OAuth callback messages
      const handleCallback = (event: MessageEvent) => {
        if (event.data?.type === 'oauth_callback') {
          // Update connection state
          const newConnection: ServiceConnectionState = {
            serviceType,
            authType: AuthType.OAuth2,
            isConfigured: true,
            isConnected: true,
            hasError: false,
            lastSync: new Date()
          };

          setConnections(prev => ({
            ...prev,
            [serviceType]: newConnection
          }));

          addServiceConnection({
            serviceType,
            authType: AuthType.OAuth2,
            isConfigured: true
          });

          setLoading(prev => ({ ...prev, [serviceType]: false }));
        } else if (event.data?.type === 'oauth_error') {
          console.error('OAuth error:', event.data.error);
          setErrors(prev => ({
            ...prev,
            [serviceType]: 'Failed to complete authentication'
          }));
          setLoading(prev => ({ ...prev, [serviceType]: false }));
        }
      };

      window.addEventListener('message', handleCallback);
      return () => window.removeEventListener('message', handleCallback);

    } catch (error) {
      console.error('Service connection error:', error);
      setErrors(prev => ({
        ...prev,
        [serviceType]: error instanceof Error ? error.message : 'Failed to initiate connection'
      }));
    } finally {
      setLoading(prev => ({ ...prev, [serviceType]: false }));
    }
  };


  const handleTestConnection = async (serviceType: ServiceType) => {
    setLoading(prev => ({ ...prev, [serviceType]: true }));
    try {
      // Test the connection by fetching basic service info
      await invoke('test_service_connection', { serviceType });

      // Update last sync time if successful
      setConnections(prev => ({
        ...prev,
        [serviceType]: {
          ...prev[serviceType],
          lastSync: new Date(),
          hasError: false
        }
      }));

    } catch (error) {
      console.error('Connection test failed:', error);
      setErrors(prev => ({
        ...prev,
        [serviceType]: 'Connection test failed'
      }));
    } finally {
      setLoading(prev => ({ ...prev, [serviceType]: false }));
    }
  };

  const handleContinue = () => {
    if (Object.values(connections).some(conn => conn.isConfigured)) {
      markStepComplete('services');
      nextStep();
    }
  };

  return (
    <Box sx={{ py: 3 }}>
      <Typography variant="h6" gutterBottom>
        Connect Your Services
      </Typography>
      <Typography color="text.secondary" component="p">
        Choose the services you want to integrate with Autoresponse. You can always add or remove services later.
      </Typography>

      <Grid container spacing={3} sx={{ mt: 2 }}>
        {Object.entries(SERVICE_METADATA).map(([type, metadata]) => {
          const serviceType = type as ServiceType;
          const connection = connections[serviceType];
          const isLoading = loading[serviceType];
          const error = errors[serviceType];

          return (
            <Grid key={type}>
              <Card>
                <CardContent>
                  <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                    <Box sx={{ mr: 1 }}>
                      {/* Replace with proper icon component */}
                      <span className={`service-icon ${metadata.icon}`} />
                    </Box>
                    <Typography variant="h6" component="div">
                      {metadata.name}
                    </Typography>
                  </Box>

                  <Typography variant="body2" color="text.secondary">
                    {metadata.description}
                  </Typography>

                  {error && (
                    <Alert severity="error" sx={{ mt: 2 }}>
                      {error}
                    </Alert>
                  )}

                  {connection?.isConnected && (
                    <Box sx={{ mt: 2, display: 'flex', alignItems: 'center' }}>
                      <CheckCircleIcon color="success" sx={{ mr: 1 }} />
                      <Typography variant="body2">
                        Connected
                        {connection.lastSync && ` - Last synced ${new Date(connection.lastSync).toLocaleString()}`}
                      </Typography>
                    </Box>
                  )}
                </CardContent>

                <CardActions>
                  <Button
                    size="small"
                    variant={connection?.isConnected ? "outlined" : "contained"}
                    onClick={(e) => {
                      e.preventDefault();
                      handleConnectService(serviceType);
                    }}
                    disabled={isLoading || isHandlingAuth}
                  >
                    {isLoading ? (
                      <CircularProgress size={20} />
                    ) : connection?.isConnected ? (
                      'Reconnect'
                    ) : (
                      'Connect'
                    )}
                  </Button>


                  {connection?.isConnected && (
                    <Tooltip title="Test connection">
                      <IconButton
                        size="small"
                        onClick={() => handleTestConnection(serviceType)}
                        disabled={isLoading}
                      >
                        <RefreshIcon />
                      </IconButton>
                    </Tooltip>
                  )}
                </CardActions>
              </Card>
            </Grid>
          );
        })}
      </Grid>

      <Box sx={{ mt: 4, display: 'flex', justifyContent: 'space-between' }}>
        <Typography variant="body2" color="text.secondary">
          You must connect at least one service to continue
        </Typography>
        <Button
          variant="contained"
          onClick={handleContinue}
          disabled={!Object.values(connections).some(conn => conn.isConfigured)}
        >
          Continue
        </Button>
      </Box>
    </Box>
  );
};

export default ServicesStep;
