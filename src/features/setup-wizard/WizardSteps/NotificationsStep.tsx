import React, { useState } from 'react';
import {
  Box,
  Typography,
  Button,
  FormGroup,
  FormControlLabel,
  Switch,
  Slider,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Paper,
  Stack,
  Alert,
} from '@mui/material';
import { useSetupWizard } from '../SetupWizardProvider';
import { invoke } from '@tauri-apps/api/core';

interface NotificationPreferences {
  desktopNotifications: boolean;
  soundEnabled: boolean;
  notificationPriority: 'all' | 'important' | 'urgent';
  autoArchiveDelay: number;
  workingHours: {
    start: number;
    end: number;
  };
  quietHours: {
    enabled: boolean;
    start: number;
    end: number;
  };
}

const NotificationsStep: React.FC = () => {
  const { nextStep, markStepComplete } = useSetupWizard();
  const [preferences, setPreferences] = useState<NotificationPreferences>({
    desktopNotifications: true,
    soundEnabled: true,
    notificationPriority: 'important',
    autoArchiveDelay: 24,
    workingHours: {
      start: 9,
      end: 17,
    },
    quietHours: {
      enabled: false,
      start: 22,
      end: 7,
    },
  });

  const handleSavePreferences = async () => {
    try {
      // Save notification preferences to backend
      await invoke('save_notification_preferences', {
        preferences,
      });

      markStepComplete('notifications');
      nextStep();
    } catch (error) {
      console.error('Failed to save notification preferences:', error);
    }
  };

  const formatHours = (value: number) => {
    const hours = Math.floor(value);
    const minutes = Math.round((value - hours) * 60);
    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}`;
  };

  return (
    <Box sx={{ py: 3 }}>
      <Typography variant="h6" gutterBottom>
        Notification Preferences
      </Typography>
      <Typography color="text.secondary" component="p">
        Configure how you want to receive and manage notifications.
      </Typography>

      <Stack spacing={4}>
        <Paper sx={{ p: 3 }}>
          <Typography variant="subtitle1" gutterBottom>
            Basic Settings
          </Typography>
          <FormGroup>
            <FormControlLabel
              control={
                <Switch
                  checked={preferences.desktopNotifications}
                  onChange={(e) =>
                    setPreferences((prev) => ({
                      ...prev,
                      desktopNotifications: e.target.checked,
                    }))
                  }
                />
              }
              label="Enable Desktop Notifications"
            />
            <FormControlLabel
              control={
                <Switch
                  checked={preferences.soundEnabled}
                  onChange={(e) =>
                    setPreferences((prev) => ({
                      ...prev,
                      soundEnabled: e.target.checked,
                    }))
                  }
                />
              }
              label="Enable Notification Sounds"
            />
          </FormGroup>
        </Paper>

        <Paper sx={{ p: 3 }}>
          <Typography variant="subtitle1" gutterBottom>
            Notification Priority
          </Typography>
          <FormControl fullWidth>
            <InputLabel>Show Notifications For</InputLabel>
            <Select
              value={preferences.notificationPriority}
              label="Show Notifications For"
              onChange={(e) =>
                setPreferences((prev) => ({
                  ...prev,
                  notificationPriority: e.target.value as any,
                }))
              }
            >
              <MenuItem value="all">All Notifications</MenuItem>
              <MenuItem value="important">Important and Urgent Only</MenuItem>
              <MenuItem value="urgent">Urgent Only</MenuItem>
            </Select>
          </FormControl>
        </Paper>

        <Paper sx={{ p: 3 }}>
          <Typography variant="subtitle1" gutterBottom>
            Auto-Archive Settings
          </Typography>
          <Typography variant="body2" color="text.secondary" gutterBottom>
            Automatically archive read notifications after:
          </Typography>
          <Box sx={{ px: 2 }}>
            <Slider
              value={preferences.autoArchiveDelay}
              onChange={(_, value) =>
                setPreferences((prev) => ({
                  ...prev,
                  autoArchiveDelay: value as number,
                }))
              }
              step={12}
              marks
              min={12}
              max={72}
              valueLabelDisplay="auto"
              valueLabelFormat={(value) => `${value} hours`}
            />
          </Box>
        </Paper>

        <Paper sx={{ p: 3 }}>
          <Typography variant="subtitle1" gutterBottom>
            Working Hours
          </Typography>
          <Box sx={{ px: 2 }}>
            <Typography variant="body2" color="text.secondary" gutterBottom>
              Active Hours:
            </Typography>
            <Slider
              value={[preferences.workingHours.start, preferences.workingHours.end]}
              onChange={(_, value) =>
                setPreferences((prev) => ({
                  ...prev,
                  workingHours: {
                    start: (value as number[])[0],
                    end: (value as number[])[1],
                  },
                }))
              }
              step={0.5}
              marks
              min={0}
              max={24}
              valueLabelDisplay="auto"
              valueLabelFormat={formatHours}
            />
          </Box>

          <Box sx={{ mt: 3 }}>
            <FormControlLabel
              control={
                <Switch
                  checked={preferences.quietHours.enabled}
                  onChange={(e) =>
                    setPreferences((prev) => ({
                      ...prev,
                      quietHours: {
                        ...prev.quietHours,
                        enabled: e.target.checked,
                      },
                    }))
                  }
                />
              }
              label="Enable Quiet Hours"
            />
            {preferences.quietHours.enabled && (
              <Box sx={{ px: 2, mt: 2 }}>
                <Typography variant="body2" color="text.secondary" gutterBottom>
                  Quiet Hours:
                </Typography>
                <Slider
                  value={[preferences.quietHours.start, preferences.quietHours.end]}
                  onChange={(_, value) =>
                    setPreferences((prev) => ({
                      ...prev,
                      quietHours: {
                        ...prev.quietHours,
                        start: (value as number[])[0],
                        end: (value as number[])[1],
                      },
                    }))
                  }
                  step={0.5}
                  marks
                  min={0}
                  max={24}
                  valueLabelDisplay="auto"
                  valueLabelFormat={formatHours}
                />
              </Box>
            )}
          </Box>
        </Paper>

        <Alert severity="info">
          These settings can be adjusted at any time from the settings menu.
        </Alert>

        <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 2 }}>
          <Button variant="contained" onClick={handleSavePreferences}>
            Save and Continue
          </Button>
        </Box>
      </Stack>
    </Box>
  );
};

export default NotificationsStep;
