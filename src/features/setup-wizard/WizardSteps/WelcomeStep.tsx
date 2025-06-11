import { Box, Typography, Button, Paper } from '@mui/material';
import React from 'react';
import { useSetupWizard } from '../SetupWizardProvider';

const WelcomeStep: React.FC = () => {
  const { nextStep, markStepComplete } = useSetupWizard();

  const handleContinue = () => {
    markStepComplete('welcome');
    nextStep();
  };

  return (
    <Paper
      elevation={3}
      sx={{
        p: 4,
        maxWidth: 600,
        mx: 'auto',
        my: 4,
        textAlign: 'center'
      }}
    >
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" component="h1" gutterBottom>
          Welcome to Autoresponse
        </Typography>
        <Typography variant="h6" color="text.secondary" gutterBottom>
          Your Unified Notification Center
        </Typography>
      </Box>

      <Box sx={{ mb: 4 }}>
        <Typography component="p">
          Let's get started by setting up your personal notification management system.
          We'll guide you through:
        </Typography>

        <Box sx={{ textAlign: 'left', my: 3 }}>
          <Typography component="ul" sx={{ listStyle: 'none', p: 0 }}>
            {[
              'Connecting your services (Github, Gmail, etc.)',
              'Configuring notification preferences',
              'Setting up AI-powered auto-responses',
              'Customizing your workflow'
            ].map((item, index) => (
              <Typography
                component="li"
                key={index}
                sx={{
                  py: 1,
                  display: 'flex',
                  alignItems: 'center',
                  '&:before': {
                    content: '"✓"',
                    color: 'success.main',
                    mr: 2,
                    fontWeight: 'bold'
                  }
                }}
              >
                {item}
              </Typography>
            ))}
          </Typography>
        </Box>
      </Box>

      <Box sx={{ mt: 4 }}>
        <Typography component="p" color="text.secondary">
          The setup process will take about 5 minutes.
        </Typography>
        <Button
          variant="contained"
          size="large"
          onClick={handleContinue}
          sx={{ minWidth: 200 }}
        >
          Let's Get Started
        </Button>
      </Box>
    </Paper>
  );
};

export default WelcomeStep;
