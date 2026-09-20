import React from 'react';
import ReactDOM from 'react-dom/client';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import App from './App';
import { Picker } from './components/Picker';
import './styles.css';

// one bundle, two windows: the main app and the transient quick picker
let isPicker = false;
try {
  isPicker = getCurrentWebviewWindow().label === 'picker';
} catch {
  /* no Tauri runtime (browser preview) — render the main app */
}

/** A render error must never leave an empty (black) window behind: show
 *  the message so the cause can be reported instead of guessed. */
class Boundary extends React.Component<{ children: React.ReactNode }, { err: string | null }> {
  state = { err: null as string | null };
  static getDerivedStateFromError(e: unknown) {
    return { err: e instanceof Error ? `${e.name}: ${e.message}` : String(e) };
  }
  render() {
    if (this.state.err) {
      return (
        <div className="fatal">
          <div className="fatal-title">clipon: Fehler beim Start · startup error</div>
          <pre>{this.state.err}</pre>
        </div>
      );
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Boundary>{isPicker ? <Picker /> : <App />}</Boundary>
  </React.StrictMode>
);
