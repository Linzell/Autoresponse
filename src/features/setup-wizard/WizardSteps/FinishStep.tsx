import React, { useState } from 'react';
import {
  Box,
  Typography,
  Button,
  Paper,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  CircularProgress,
  Alert,
  Collapse,
} from '@mui/material';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import SettingsIcon from '@mui/icons-material/Settings';
import NotificationsIcon from '@mui/icons-material/Notifications';
import SmartToyIcon from '@mui/icons-material/SmartToy';
import { useSetupWizard } from '../SetupWizardProvider';
import { TransitionGroup } from 'react-transition-group';
import { invoke } from '@tauri-apps/api/core';

const FinishStep: React.FC = () => {
  const { steps, configuredServices, completeSetup } = useSetupWizard();
  const [finalizing, setFinalizing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [completedItems, setCompletedItems] = useState<string[]>([]);

  const finalizationSteps = [
    {
      id: 'services',
      label: 'Finalizing service connections',
      icon: <SettingsIcon />,
    },
    {
      id: 'notifications',
      label: 'Setting up notification handlers',
      icon: <NotificationsIcon />,
    },
    {
      id: 'ai',
      label: 'Initializing AI components',
      icon: <SmartToyIcon />,
    },
  ];

  const handleFinish = async () => {
    setFinalizing(true);
    setError(null);
    setCompletedItems([]);

    try {
      // Simulate finalizing each component
      for (const step of finalizationSteps) {
        await new Promise((resolve) => setTimeout(resolve, 1000));
        setCompletedItems((prev) => [...prev, step.id]);
      }

      // Complete the setup
      await invoke('complete_setup');
      completeSetup();

    } catch (err) {
      setError('Failed to complete setup. Please try again.');
      console.error('Setup completion error:', err);
    } finally {
      setFinalizing(false);
    }
  };

  return (
    <Box sx={{ py: 3 }}>
      <Typography variant="h6" gutterBottom>
        Setup Complete!
      </Typography>
      <Typography color="text.secondary" component="p">
        You've successfully configured Autoresponse. Here's a summary of your setup:
      </Typography>

      <Paper sx={{ p: 3, mb: 3 }}>
        <Typography variant="subtitle1" gutterBottom>
          Configured Services ({configuredServices.length})
        </Typography>
        <List>
          {configuredServices.map((service) => (
            <ListItem key={service.serviceType}>
              <ListItemIcon>
                <CheckCircleIcon color="success" />
              </ListItemIcon>
              <ListItemText
                primary={service.serviceType}
                secondary={`Authentication: ${service.authType}`}
              />
            </ListItem>
          ))}
        </List>
      </Paper>

      {finalizing && (
        <Paper sx={{ p: 3, mb: 3 }}>
          <Typography variant="subtitle1" gutterBottom>
            Finalizing Setup
          </Typography>
          <List>
            <TransitionGroup>
              {finalizationSteps.map((step) => (
                <Collapse key={step.id}>
                  <ListItem>
                    <ListItemIcon>
                      {completedItems.includes(step.id) ? (
                        <CheckCircleIcon color="success" />
                      ) : (
                        <CircularProgress size={24} />
                      )}
                    </ListItemIcon>
                    <ListItemText primary={step.label} />
                  </ListItem>
                </Collapse>
              ))}
            </TransitionGroup>
          </List>
        </Paper>
      )}

      {error && (
        <Alert severity="error" sx={{ mb: 3 }}>
          {error}
        </Alert>
      )}

      <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 4 }}>
        <Button
          variant="contained"
          onClick={handleFinish}
          disabled={finalizing}
          size="large"
        >
          {finalizing ? (
            <>
              <CircularProgress size={20} sx={{ mr: 1 }} />
              Finalizing...
            </>
          ) : (
            'Start Using Autoresponse'
          )}
        </Button>
      </Box>

      <Box sx={{ mt: 4 }}>
        <Typography variant="body2" color="text.secondary">
          Need help? Check out our documentation or contact support.
        </Typography>
      </Box>
    </Box>
  );
};

export default FinishStep;
