import { Component } from 'preact';
import moment from 'moment';

class LogStream extends Component {
  constructor (props) {
    super(props)
    this.logContainerEl = null;
    this.shouldScroll = true;
  }

  componentDidMount() { this.scroll(); }
  componentDidUpdate() { this.scroll(); }

  scroll() {
    if (this.shouldScroll) {
      this.logContainerEl.scrollTop = 99999999;
    }
  }

  render() {
    const searchExpr = this.props.filter || '';
    const searchTokens = searchExpr.toLowerCase().trim().split(/\s+/).filter(x => x);
    const searchPattern = new RegExp('(' + searchTokens.join('|') + ')', 'ig');

    const el = this.logContainerEl;
    this.shouldScroll = !el || (el.scrollHeight - el.scrollTop) === el.offsetHeight;

    return (
      <main>
        <ol id="logs" ref={el => { this.logContainerEl = el; }}>
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
