import { Component } from 'preact';
import Header from './Header';
import LogStream from './LogStream';
import Footer from './Footer';
import Landing from './Landing';
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
      streamError: false,
      suspended: null,
      showLanding: !this.bucketID
    }
  }

  componentDidMount() {
    // Only connect to stream if we have a bucket ID
    if (!this.bucketID) return;

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
    this.stream.on('suspension', suspension => this.setState({ suspended: suspension }));
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

  handleGetStarted = () => {
    // Redirect to create a new random bucket
    window.location.href = '/new';
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
    // Show landing page if no bucket ID
    if (this.state.showLanding) {
      return <Landing onGetStarted={this.handleGetStarted} />;
    }

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
        {this.state.suspended && this.state.suspended.suspended && (
          <div className='error-modal suspended-modal'>
            <div className='heading'>🚫 Bin Suspended</div>
            <p>This bin has been suspended due to high traffic volumes.</p>
            <p><code>log-bin</code> is intended for development and debugging purposes, and is not designed to handle high volumes of traffic. If you need to inspect logs for a production workload or have any questions about this suspension, please contact Fastly support.</p>
          </div>
        )}
        <Footer bucketID={this.bucketID} />
      </div>
    );
  }
}

export default Main;
