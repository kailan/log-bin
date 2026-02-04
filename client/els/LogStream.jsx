import { Component } from 'preact';
import moment from 'moment';

class LogStream extends Component {
  constructor (props) {
    super(props)
    this.logContainerEl = null;
    this.shouldScroll = true;
    this.state = {
      copied: false
    };
  }

  componentDidMount() { this.scroll(); }
  componentDidUpdate() { this.scroll(); }

  scroll() {
    if (this.shouldScroll) {
      this.logContainerEl.scrollTop = 99999999;
    }
  }

  copyToClipboard(text) {
    navigator.clipboard.writeText(text);
    this.setState({ copied: true });
    setTimeout(() => this.setState({ copied: false }), 2000);
  }

  render() {
    const searchExpr = this.props.filter || '';
    const searchTokens = searchExpr.toLowerCase().trim().split(/\s+/).filter(x => x);
    const searchPattern = new RegExp('(' + searchTokens.join('|') + ')', 'ig');

    const el = this.logContainerEl;
    this.shouldScroll = !el || (el.scrollHeight - el.scrollTop) === el.offsetHeight;

    const hasLogs = this.props.events.length > 0;
    const bucketUrl = window.location.href.split('?')[0];
    const curlCommand = "curl -X POST " + bucketUrl + " -d 'Hello from the terminal!'";

    return (
      <main>
        {!hasLogs && (
          <div className="empty-state">
            <div className="empty-state-icon">
              <div className="pulse-ring"></div>
              <div className="pulse-dot"></div>
            </div>
            <h2>Waiting for logs...</h2>
            <p>Send logs to this bin by POSTing to:</p>
            <div className="empty-state-url">
              <code>{bucketUrl}</code>
            </div>
            <div className="empty-state-example">
              <div className="example-header">
                <span>Try it:</span>
                <button
                  className={`copy-btn ${this.state.copied ? 'copied' : ''}`}
                  onClick={() => this.copyToClipboard(curlCommand)}
                >
                  {this.state.copied ? '✓ Copied' : 'Copy'}
                </button>
              </div>
              <pre><code>{curlCommand}</code></pre>
            </div>
          </div>
        )}
        <ol id="logs" ref={el => { this.logContainerEl = el; }} className={!hasLogs ? 'hidden' : ''}>
          {this.props.events.map((evt, idx, allEvts) => {

            let isHidden, msgHTML;
            if (searchTokens.length) {
              isHidden = !evt.raw.match(searchPattern);
              msgHTML = {__html: evt.raw.replace(searchPattern, '<span class="search-highlight">$1</span>')};
            }
            const isSep = idx > 0 && evt.time > (allEvts[idx-1].time + 3000);

            return (
              <li key={idx} className={(isHidden ? 'hidden' : '') + (isSep ? ' separator' : '')}>
                <span class='timestamp'>{evt.timeString}</span>
                {Boolean(msgHTML) ? (
                  <span class='message' dangerouslySetInnerHTML={msgHTML}></span>
                ) : Boolean(evt.message) ? (
                  <span class='message'>{evt.message}</span>
                ) : ''}
                {evt.fields && (
                  <ul class='meta'>
                    {Object.entries(evt.fields)
                      .map(([key, item]) => (
                        <li key={key}>
                          <label title={key}>
                            <i style={{backgroundColor: item.color}} />
                          </label>
                          {item.value}
                        </li>
                      ))}
                  </ul>
                )}
              </li>
            );
          })}
        </ol>
      </main>
    );
  }
}

export default LogStream;
