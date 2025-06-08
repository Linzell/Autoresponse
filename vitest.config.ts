import { defineConfig, mergeConfig } from 'vitest/config';
import { fileURLToPath } from 'url';
import viteConfig from './vite.config';
import storybookTest from '@storybook/addon-vitest/vitest-plugin';

export default defineConfig(configEnv => mergeConfig(
  viteConfig(configEnv), defineConfig({
    plugins: [
      // The plugin will run tests for the stories defined in your Storybook config
      // See options at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon#storybooktest
      storybookTest({ configDir: fileURLToPath(new URL('./.storybook', import.meta.url)) }),
    ],
    test: {
      projects: [
        {
          test: {
            name: { label: 'storybook', color: 'magenta' },
            exclude: [
              'node_modules',
              'tests'
            ],
            browser: {
              enabled: true,
              headless: true,
              provider: 'playwright',
              instances: [{ browser: 'chromium' }]
            },
            setupFiles: ['.storybook/vitest.setup.ts'],
            globals: true
          }
        },
        {
          test: {
            name: { label: 'node', color: 'green' },
            include: ['**/*.{test,spec}.?(c|m)[jt]s?(x)'],
            exclude: [
              'node_modules',
              './tests/setup.ts',
              '*.stories.tsx'
            ],
            environment: 'jsdom',
            setupFiles: ['./tests/setup.ts'],
            globals: true
          }
        }
      ]
    }
  })
))
