import React from 'react';
import ReactDOM from 'react-dom/client';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import App from './App';
import { Picker } from './components/Picker';
import './styles.css';

// one bundle, two windows: the main app and the transient quick picker
const isPicker = getCurrentWebviewWindow().label === 'picker';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>{isPicker ? <Picker /> : <App />}</React.StrictMode>
);
