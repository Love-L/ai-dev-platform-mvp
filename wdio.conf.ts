export const config = {
  runner: 'local',
  specs: [
    './tests/e2e/**/*.test.ts'
  ],
  maxInstances: 1,
  capabilities: [{
    browserName: 'tauri',
    'wdio:tauriOptions': {
      driverProvider: 'external',
      tauriOptions: {
        application: '../../src-tauri/target/debug/ai-dev-platform-mvp.exe',
      }
    }
  }],
  logLevel: 'warn',
  bail: 0,
  baseUrl: 'http://localhost:1420',
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,
  services: [
    ['tauri', {
      driverProvider: 'external',
      tauriOptions: {
        application: '../../src-tauri/target/debug/ai-dev-platform-mvp.exe',
      }
    }]
  ],
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000
  }
};
