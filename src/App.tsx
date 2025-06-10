import "./App.css";
import { useState, useEffect } from "react";
import { ThemeProvider, CssBaseline } from "@mui/material";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SetupWizardProvider } from "./features/setup-wizard/SetupWizardProvider";
import SetupWizard from "./features/setup-wizard/SetupWizard";
import { theme } from "./theme";
import { invoke } from "@tauri-apps/api/core";

const queryClient = new QueryClient();

function App() {
  const [isFirstRun, setIsFirstRun] = useState<boolean | null>(null);

  useEffect(() => {
    const checkFirstRun = async () => {
      try {
        const result = await invoke('check_first_run');
        setIsFirstRun(result as boolean);
      } catch (error) {
        console.error('Failed to check first run status:', error);
        setIsFirstRun(false);
      }
    };

    checkFirstRun();
  }, []);

  if (isFirstRun === null) {
    return null; // Loading state
  }

  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        {isFirstRun ? (
          <SetupWizardProvider>
            <SetupWizard />
          </SetupWizardProvider>
        ) : (
          <main className="container">
            {/* Main application content will go here */}
          </main>
        )}
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export default App;
