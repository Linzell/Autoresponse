import React from 'react';
import { Box, Paper, Stepper, Step, StepLabel, StepContent, Button } from '@mui/material';
import { useSetupWizard } from './SetupWizardProvider';
import {
  WelcomeStep,
  ServicesStep,
  NotificationsStep,
  AIConfigStep,
  FinishStep,
} from './WizardSteps';

const SetupWizard: React.FC = () => {
  const {
    currentStep,
    steps,
    previousStep,
    isSetupComplete,
  } = useSetupWizard();

  const handleBack = () => {
    previousStep();
  };

  const getStepContent = (step: number) => {
    switch (step) {
      case 0:
        return <WelcomeStep />;
      case 1:
        return <ServicesStep />;
      case 2:
        return <NotificationsStep />;
      case 3:
        return <AIConfigStep />;
      case 4:
        return <FinishStep />;
      default:
        return null;
    }
  };

  if (isSetupComplete) {
    return null;
  }

  return (
    <Box sx={{ 
      minHeight: '100vh',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      bgcolor: 'background.default',
      p: 3 
    }}>
      <Paper 
        elevation={3}
        sx={{ 
          maxWidth: 900,
          width: '100%',
          mx: 'auto',
          p: { xs: 2, sm: 3, md: 4 }
        }}
      >
        <Stepper 
          activeStep={currentStep} 
          orientation="vertical"
          sx={{ mb: 3 }}
        >
          {steps.map((step, index) => (
            <Step key={step.id}>
              <StepLabel sx={{ py: 1 }}>
                {step.title}
              </StepLabel>
              <StepContent>
                {getStepContent(index)}
                <Box sx={{ mb: 2 }}>
                  {currentStep !== 0 && (
                    <Button
                      onClick={handleBack}
                      sx={{ mt: 1, mr: 1 }}
                    >
                      Back
                    </Button>
                  )}
                </Box>
              </StepContent>
            </Step>
          ))}
        </Stepper>
      </Paper>
    </Box>
  );
};

export default SetupWizard;