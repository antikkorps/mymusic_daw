/**
 * Example App.tsx for MyMusic DAW React Frontend
 * Copy this to your Vite project's src/App.tsx
 */

import { DawEngineTest } from './components/DawEngineTest';

function App() {
  return (
    <div style={styles.app}>
      <DawEngineTest />
    </div>
  );
}

const styles = {
  app: {
    minHeight: '100vh',
    backgroundColor: '#0f0f0f',
    padding: '20px',
  },
};

export default App;
