import { Component } from 'preact';
import fastlyLogo from '../speedometer-white.svg';

const DEMO_LOGS = [
  { time: '14:32:01.234', msg: 'GET /api/users - 200 OK (12ms)' },
  { time: '14:32:01.456', msg: 'Cache HIT for /static/app.js' },
  { time: '14:32:02.012', msg: '{"level":"info","msg":"User authenticated","user_id":"usr_8x7k2"}' },
  { time: '14:32:02.789', msg: 'POST /api/webhook received' },
  { time: '14:32:03.111', msg: 'Edge compute: request processed in 3ms' },
  { time: '14:32:03.555', msg: '{"level":"warn","msg":"Rate limit approaching","remaining":12}' },
  { time: '14:32:04.001', msg: 'Cache MISS - fetching from origin' },
  { time: '14:32:04.234', msg: 'VCL: beresp.ttl = 3600s' },
];

class AnimatedDemo extends Component {
  constructor(props) {
    super(props);
    this.state = {
      visibleLogs: [],
      currentIndex: 0
    };
  }

  componentDidMount() {
    this.addNextLog();
  }

  componentWillUnmount() {
    clearTimeout(this.timeout);
  }

  addNextLog = () => {
    const { currentIndex } = this.state;

    if (currentIndex < DEMO_LOGS.length) {
      this.setState(state => ({
        visibleLogs: [...state.visibleLogs, DEMO_LOGS[state.currentIndex]],
        currentIndex: state.currentIndex + 1
      }));
      this.timeout = setTimeout(this.addNextLog, 800 + Math.random() * 600);
    }
    // Animation complete - don't loop
  }

  render() {
    const { visibleLogs } = this.state;

    return (
      <div className="demo-window">
        <div className="demo-titlebar">
          <div className="demo-titlebar-dots">
            <span className="dot red"></span>
            <span className="dot yellow"></span>
            <span className="dot green"></span>
          </div>
          <div className="demo-titlebar-title">my-debug-session — log-bin</div>
        </div>
        <div className="demo-header">
          <span className="demo-bucket">my-debug-session</span>
          <span className="demo-conn-count">2</span>
          <input type="text" placeholder="Type to filter" disabled className="demo-filter" />
        </div>
        <div className="demo-logs">
          {visibleLogs.map((log, i) => (
            <div key={i} className={`demo-log-entry ${i === visibleLogs.length - 1 ? 'new' : ''}`}>
              <span className="demo-time">{log.time}</span>
              <span className="demo-msg">{log.msg}</span>
            </div>
          ))}
          <div className="demo-cursor">▋</div>
        </div>
      </div>
    );
  }
}

class Landing extends Component {
  constructor(props) {
    super(props);
    this.state = {
      copied: null
    };
  }

  copyToClipboard(text, id) {
    navigator.clipboard.writeText(text);
    this.setState({ copied: id });
    setTimeout(() => this.setState({ copied: null }), 2000);
  }

  render() {
    const { copied } = this.state;
    const exampleBucket = 'my-debug-session';
    const baseUrl = window.location.origin;

    return (
      <div className="landing">
        <header className="landing-header">
          <a href="/" className="landing-header-brand">
            <img src={fastlyLogo} alt="Fastly" className="fastly-logo" />
            <span className="landing-header-title">log-bin</span>
          </a>
        </header>

        <div className="warning-banner">
          <strong>⚠️ NOT FOR PRODUCTION USE</strong> — This is a debugging sandbox. Do not send confidential or sensitive data.
          Logs are accessible to anyone who has the link.
        </div>

        <div className="demo-hero">
          <AnimatedDemo />
        </div>

        <div className="landing-hero">
          <h1 className="landing-headline">
            A quick and simple log viewer for debugging
          </h1>
          <p className="landing-description">
            Pipe logs from anywhere to your browser. Useful for debugging edge functions,
            webhooks, or anything where you can't easily tail a log file.
          </p>
          <button className="cta-button" onClick={this.props.onGetStarted}>
            Create a throwaway bin →
          </button>
        </div>

        <div className="disclaimer-box">
          <h3>🚨 Read this first</h3>
          <ul className="disclaimer-list">
            <li>
              <svg className="disclaimer-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
                <circle cx="12" cy="12" r="3"></circle>
              </svg>
              <span><strong>Anyone with the URL can see your logs.</strong> Bin names are the only "security".</span>
            </li>
            <li>
              <svg className="disclaimer-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="3 6 5 6 21 6"></polyline>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
              <span><strong>Logs are not persisted.</strong> Close the browser tab and they may be gone forever.</span>
            </li>
            <li>
              <svg className="disclaimer-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="10"></circle>
                <polyline points="12 6 12 12 16 14"></polyline>
              </svg>
              <span><strong>Rate limited.</strong> High traffic bins get automatically suspended.</span>
            </li>
            <li>
              <svg className="disclaimer-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
                <line x1="12" y1="9" x2="12" y2="13"></line>
                <line x1="12" y1="17" x2="12.01" y2="17"></line>
              </svg>
              <span><strong>Zero guarantees.</strong> This is a free tool. It might break. Don't rely on it.</span>
            </li>
          </ul>
        </div>

        <div className="how-it-works">
          <h2>How it works</h2>

          <div className="step">
            <div className="step-label">1. Open a bin</div>
            <p>Visit any URL to create a bin. Pick something random so others can't guess it.</p>
            <div className="code-block">
              <code>{baseUrl}/<span className="hl">{exampleBucket}</span></code>
            </div>
          </div>

          <div className="step">
            <div className="step-label">2. POST your logs</div>
            <p>Send log lines to the same URL. One line per log entry.</p>

            <div className="code-example">
              <div className="code-header">
                <span>cURL</span>
                <button
                  className={`copy-btn ${copied === 'curl' ? 'copied' : ''}`}
                  onClick={() => this.copyToClipboard(`curl -X POST ${baseUrl}/${exampleBucket} -d "something happened"`, 'curl')}
                >
                  {copied === 'curl' ? '✓' : 'copy'}
                </button>
              </div>
              <pre><code>{`curl -X POST ${baseUrl}/${exampleBucket} \\
  -d "something happened"`}</code></pre>
            </div>

            <div className="code-example">
              <div className="code-header">
                <span>JavaScript</span>
                <button
                  className={`copy-btn ${copied === 'js' ? 'copied' : ''}`}
                  onClick={() => this.copyToClipboard(`fetch('${baseUrl}/${exampleBucket}', { method: 'POST', body: 'debug: ' + JSON.stringify(data) })`, 'js')}
                >
                  {copied === 'js' ? '✓' : 'copy'}
                </button>
              </div>
              <pre><code>{`fetch('${baseUrl}/${exampleBucket}', {
  method: 'POST',
  body: 'debug: ' + JSON.stringify(data)
})`}</code></pre>
            </div>

            <div className="code-example">
              <div className="code-header">
                <span>Fastly VCL</span>
                <button
                  className={`copy-btn ${copied === 'vcl' ? 'copied' : ''}`}
                  onClick={() => this.copyToClipboard(`log "syslog " req.service_id " log-bin :: " req.url;`, 'vcl')}
                >
                  {copied === 'vcl' ? '✓' : 'copy'}
                </button>
              </div>
              <pre><code>{`# Add log-bin as an HTTPS log endpoint, then:
log "syslog " req.service_id " log-bin :: " req.url;`}</code></pre>
            </div>
          </div>

          <div className="step">
            <div className="step-label">3. Watch them appear</div>
            <p>Logs stream to the browser in real-time. Use the filter box to search.</p>
          </div>
        </div>

        <div className="use-cases">
          <h2>Good for</h2>
          <ul>
            <li>Debugging Fastly VCL / Compute services</li>
            <li>Debugging Cloudflare Workers, Lambda@Edge, etc.</li>
            <li>Inspecting webhook payloads</li>
            <li>Quick debugging when you can't SSH in</li>
            <li>Sharing debug output with a colleague</li>
          </ul>

          <h2>Not for</h2>
          <ul className="not-for">
            <li>Production logging (use a real logging service)</li>
            <li>Anything with sensitive data</li>
            <li>High-volume log streams</li>
            <li>Long-term log storage</li>
          </ul>
        </div>

        <div className="cta-section">
          <button className="cta-button" onClick={this.props.onGetStarted}>
            Create a bin →
          </button>
          <p className="cta-note">Or just visit <code>{baseUrl}/your-bin-name</code></p>
        </div>
      </div>
    );
  }
}

export default Landing;
