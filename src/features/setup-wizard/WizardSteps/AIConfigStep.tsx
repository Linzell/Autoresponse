import React, { useState } from 'react';
import {
  Box,
  Typography,
  Button,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  TextField,
  Paper,
  Stack,
  Slider,
  FormControlLabel,
  Switch,
  Alert,
  CircularProgress,
} from '@mui/material';
import { useSetupWizard } from '../SetupWizardProvider';
import { invoke } from '@tauri-apps/api/core';

interface AIConfig {
  model: string;
  temperature: number;
  maxTokens: number;
  responseStyle: 'professional' | 'casual' | 'friendly';
  autoResponseEnabled: boolean;
  customPrompt: string;
  useLocalModel: boolean;
  localModelPath?: string;
  fallbackToCloud: boolean;
}

const AIConfigStep: React.FC = () => {
  const { nextStep, markStepComplete } = useSetupWizard();
  const [testing, setTesting] = useState(false);
  const [testError, setTestError] = useState<string | null>(null);
  const [config, setConfig] = useState<AIConfig>({
    model: 'gpt-3.5-turbo',
    temperature: 0.7,
    maxTokens: 150,
    responseStyle: 'professional',
    autoResponseEnabled: true,
    customPrompt: '',
    useLocalModel: true,
    localModelPath: '',
    fallbackToCloud: true,
  });

  const availableModels = [
    { id: 'gpt-3.5-turbo', name: 'GPT-3.5 Turbo' },
    { id: 'llama2', name: 'Llama 2' },
    { id: 'mistral', name: 'Mistral' },
    { id: 'custom', name: 'Custom Model' },
  ];

  const handleSaveConfig = async () => {
    try {
      await invoke('save_ai_config', { config });
      markStepComplete('ai');
      nextStep();
    } catch (error) {
      console.error('Failed to save AI configuration:', error);
    }
  };

  const handleTestConnection = async () => {
    setTesting(true);
    setTestError(null);
    try {
      await invoke('test_ai_connection', { config });
    } catch (error) {
      setTestError(error as string);
    } finally {
      setTesting(false);
    }
  };

  return (
    <Box sx={{ py: 3 }}>
      <Typography variant="h6" gutterBottom>
        AI Configuration
      </Typography>
      <Typography color="text.secondary" component="p">
        Configure how the AI should process and respond to notifications.
      </Typography>

      <Stack spacing={4}>
        <Paper sx={{ p: 3 }}>
          <Typography variant="subtitle1" gutterBottom>
            Model Settings
          </Typography>
          <FormControl fullWidth sx={{ mb: 3 }}>
            <InputLabel>Language Model</InputLabel>
            <Select
              value={config.model}
              label="Language Model"
              onChange={(e) =>
                setConfig((prev) => ({
                  ...prev,
                  model: e.target.value,
                }))
              }
            >
              {availableModels.map((model) => (
                <MenuItem key={model.id} value={model.id}>
                  {model.name}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          <FormControlLabel
            control={
              <Switch
                checked={config.useLocalModel}
                onChange={(e) =>
                  setConfig((prev) => ({
                    ...prev,
                    useLocalModel: e.target.checked,
                  }))
                }
              />
            }
            label="Use Local Model (via Ollama)"
          />

          {config.useLocalModel && (
            <>
              <FormControlLabel
                control={
                  <Switch
                    checked={config.fallbackToCloud}
                    onChange={(e) =>
                      setConfig((prev) => ({
                        ...prev,
                        fallbackToCloud: e.target.checked,
                      }))
                    }
                  />
                }
                label="Fallback to Cloud API if Local Model Fails"
              />
              <Alert severity="info" sx={{ mt: 2 }}>
                Make sure Ollama is installed and running on your system.
              </Alert>
            </>
          )}
        </Paper>

        <Paper sx={{ p: 3 }}>
          <Typography variant="subtitle1" gutterBottom>
            Response Configuration
          </Typography>
          <FormControl fullWidth sx={{ mb: 3 }}>
            <InputLabel>Response Style</InputLabel>
            <Select
              value={config.responseStyle}
              label="Response Style"
              onChange={(e) =>
                setConfig((prev) => ({
                  ...prev,
                  responseStyle: e.target.value as any,
                }))
              }
            >
              <MenuItem value="professional">Professional</MenuItem>
              <MenuItem value="casual">Casual</MenuItem>
              <MenuItem value="friendly">Friendly</MenuItem>
            </Select>
          </FormControl>

          <Typography variant="body2" gutterBottom>
            Temperature (Creativity)
          </Typography>
          <Slider
            value={config.temperature}
            onChange={(_, value) =>
              setConfig((prev) => ({
                ...prev,
                temperature: value as number,
              }))
            }
            step={0.1}
            marks
            min={0}
            max={1}
            valueLabelDisplay="auto"
          />

          <Typography variant="body2" gutterBottom sx={{ mt: 2 }}>
            Maximum Response Length (Tokens)
          </Typography>
          <Slider
            value={config.maxTokens}
            onChange={(_, value) =>
              setConfig((prev) => ({
                ...prev,
                maxTokens: value as number,
              }))
            }
            step={50}
            marks
            min={50}
            max={500}
            valueLabelDisplay="auto"
          />

          <TextField
            fullWidth
            multiline
            rows={4}
            label="Custom Response Template"
            placeholder="Enter a custom template for AI responses..."
            value={config.customPrompt}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                customPrompt: e.target.value,
              }))
            }
            sx={{ mt: 3 }}
            helperText="Leave blank to use default templates"
          />
        </Paper>

        {testError && (
          <Alert severity="error">
            Failed to connect to AI service: {testError}
          </Alert>
        )}

        <Box sx={{ display: 'flex', justifyContent: 'flex-end', gap: 2 }}>
          <Button
            variant="outlined"
            onClick={handleTestConnection}
            disabled={testing}
          >
            {testing ? (
              <CircularProgress size={20} sx={{ mr: 1 }} />
            ) : (
              'Test Connection'
            )}
          </Button>
          <Button
            variant="contained"
            onClick={handleSaveConfig}
            disabled={testing}
          >
            Save and Continue
          </Button>
        </Box>
      </Stack>
    </Box>
  );
};

export default AIConfigStep;
