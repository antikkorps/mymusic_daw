import { DawEngineTest } from './components/DawEngineTest'
import { PluginParameterControl } from './components/PluginParameterControl'
import './App.css'

function App() {
  return (
    <div className="app">
      <DawEngineTest />
      <div style={{ marginTop: '30px' }}>
        <PluginParameterControl />
      </div>
    </div>
  )
}

export default App
