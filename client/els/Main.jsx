import { Component } from 'preact';
import Header from './Header';
import LogStream from './LogStream';
import Footer from './Footer';
import Stream from '../stream';

const STREAM_TIMEOUT_MS = 3000;

class Main extends Component {
  constructor (props) {
    super(props)

    this.url = new URL(location.href);
    this.bucketID = this.url.pathname.split('/')[1];
    this.errorTimer = null;
    this.stream = null;

    this.state = {
      filterText: this.url.searchParams.get('filter'),
      events: [],
      stats: {},
      streamError: false
    }
  }

  componentDidMount() {
    const opts = {};
    if (this.url.searchParams.has('msg')) opts.msgKeys = this.url.searchParams.get('msg').split(/[,|]/);
    if (this.url.searchParams.has('meta')) opts.metaKeys = this.url.searchParams.get('meta').split(/[,|]/);
    if (this.url.searchParams.has('time')) opts.timeKeys = this.url.searchParams.get('time').split(/[,|]/);
    this.stream = new Stream(this.bucketID, opts);
    this.stream.on('log', newEvent => {
      this.setState(state => {
        state.events.push(newEvent);
        return state;
      });
    });
    this.stream.on('stats', stats => this.setState({ stats }));
    this.stream.on('stateChange', newStreamState => {
      clearTimeout(this.errorTimer);
      if (newStreamState !== 'open') {
        this.errorTimer = setTimeout(() => this.setState({ streamError: true }), STREAM_TIMEOUT_MS);
      } else {
        this.setState({ streamError: false });
      }
    });
    this.stream.connect();
  }

  componentDidUpdate() {
    if (this.state.filterText) {
      this.url.searchParams.set('filter', this.state.filterText);
    } else {
      this.url.searchParams.delete('filter');
    }
    window.history.pushState({}, '', this.url.toString());
  }

  setFilter(newVal) {
    this.setState({ filterText: newVal });
  }

  render() {
    return (
      <div className='root'>
        <Header
          bucketID={this.bucketID}
          filterVal={this.state.filterText}
          onFilter={newVal => this.setFilter(newVal)}
          connCount={!this.state.streamError && this.state.stats.connCount}
          clientCount={!this.state.streamError && this.state.stats.clientCount}
          clients={this.state.stats.clients}
        />
        <LogStream
          filter={this.state.filterText}
          events={this.state.events}
        />
        {this.state.streamError && (
          <div className='error-modal'>
            <div className='heading'>Stream disconnected</div>
            <p>
              An error occured connecting to the stream.  This may happen if the stream
              has exceeded its maximum number of concurrent subscribers.
            </p>
            <p>Reload the page to retry.</p>
          </div>
        )}
        <Footer bucketID={this.bucketID} />
      </div>
    );
  }
}

export default Main;
