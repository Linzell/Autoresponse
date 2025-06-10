import React, { createContext, useContext, useState, useCallback, useMemo } from 'react';
import { AuthType, ServiceType } from '../../types/services';

export interface WizardStep {
  id: string;
  title: string;
  description: string;
  isCompleted: boolean;
}

export interface ServiceConnection {
  serviceType: ServiceType;
  authType: AuthType;
  isConfigured: boolean;
}

interface SetupWizardContextType {
  currentStep: number;
  steps: WizardStep[];
  configuredServices: ServiceConnection[];
  isSetupComplete: boolean;
  nextStep: () => void;
  previousStep: () => void;
  goToStep: (step: number) => void;
  markStepComplete: (stepId: string) => void;
  addServiceConnection: (connection: ServiceConnection) => void;
  updateServiceConnection: (serviceType: ServiceType, connection: Partial<ServiceConnection>) => void;
  completeSetup: () => void;
}

const SetupWizardContext = createContext<SetupWizardContextType | undefined>(undefined);

export const defaultSteps: WizardStep[] = [
  {
    id: 'welcome',
    title: 'Welcome to Autoresponse',
    description: 'Let\'s get started with setting up your notification center',
    isCompleted: false,
  },
  {
    id: 'services',
    title: 'Connect Your Services',
    description: 'Choose and connect the services you want to manage',
    isCompleted: false,
  },
  {
    id: 'notifications',
    title: 'Configure Notifications',
    description: 'Set up how you want to receive notifications',
    isCompleted: false,
  },
  {
    id: 'ai',
    title: 'AI Configuration',
    description: 'Configure AI response settings and preferences',
    isCompleted: false,
  },
  {
    id: 'finish',
    title: 'Ready to Go',
    description: 'Complete setup and start using Autoresponse',
    isCompleted: false,
  },
];

interface SetupWizardProviderProps {
  children: React.ReactNode;
}

export const SetupWizardProvider: React.FC<SetupWizardProviderProps> = ({ children }) => {
  const [currentStep, setCurrentStep] = useState(0);
  const [steps, setSteps] = useState<WizardStep[]>(defaultSteps);
  const [configuredServices, setConfiguredServices] = useState<ServiceConnection[]>([]);
  const [isSetupComplete, setIsSetupComplete] = useState(false);

  const nextStep = useCallback(() => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(prev => prev + 1);
    }
  }, [currentStep, steps.length]);

  const previousStep = useCallback(() => {
    if (currentStep > 0) {
      setCurrentStep(prev => prev - 1);
    }
  }, [currentStep]);

  const goToStep = useCallback((step: number) => {
    if (step >= 0 && step < steps.length) {
      setCurrentStep(step);
    }
  }, [steps.length]);

  const markStepComplete = useCallback((stepId: string) => {
    setSteps(prevSteps =>
      prevSteps.map(step =>
        step.id === stepId ? { ...step, isCompleted: true } : step
      )
    );
  }, []);

  const addServiceConnection = useCallback((connection: ServiceConnection) => {
    setConfiguredServices(prev => [...prev, connection]);
  }, []);

  const updateServiceConnection = useCallback((
    serviceType: ServiceType,
    connection: Partial<ServiceConnection>
  ) => {
    setConfiguredServices(prev =>
      prev.map(svc =>
        svc.serviceType === serviceType ? { ...svc, ...connection } : svc
      )
    );
  }, []);

  const completeSetup = useCallback(() => {
    setIsSetupComplete(true);
  }, []);

  const contextValue = useMemo(() => ({
    currentStep,
    steps,
    configuredServices,
    isSetupComplete,
    nextStep,
    previousStep,
    goToStep,
    markStepComplete,
    addServiceConnection,
    updateServiceConnection,
    completeSetup,
  }), [
    currentStep,
    steps,
    configuredServices,
    isSetupComplete,
    nextStep,
    previousStep,
    goToStep,
    markStepComplete,
    addServiceConnection,
    updateServiceConnection,
    completeSetup,
  ]);

  return (
    <SetupWizardContext.Provider value={contextValue}>
      {children}
    </SetupWizardContext.Provider>
  );
};

export const useSetupWizard = () => {
  const context = useContext(SetupWizardContext);
  if (context === undefined) {
    throw new Error('useSetupWizard must be used within a SetupWizardProvider');
  }
  return context;
};
