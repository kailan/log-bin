import { Component } from 'preact';
import fastlyLogo from '../speedometer-white.svg';

class Header extends Component {
  constructor (props) {
    super(props)
    this.state = {
      activeMenu: null
    }
  }

  openMenu(activeMenu) {
    this.setState({ activeMenu });
  }

  render() {
    return (
      <header>
        <div className='app-header'>
          <a href="/" className='app-header-brand'>
            <img src={fastlyLogo} alt="Fastly" className="fastly-logo" />
            <span className='app-header-title'>log-bin</span>
          </a>
          <div className='app-header-bucket'>
            <span className='bucket-name'>{this.props.bucketID}</span>
            {Boolean(this.props.connCount) && (
              <span className='conn-count' title='Number of connected clients'>{this.props.connCount}</span>
            )}
          </div>
          <input
            type="text"
            id="filter"
            placeholder="Type to filter"
            value={this.props.filterVal}
            onChange={evt => this.props.onFilter(evt.target.value)}
          />
        </div>
        <div className='warning-banner'>
          <strong>⚠️ NOT FOR PRODUCTION USE</strong> — Do not send confidential or sensitive data.
          Logs are accessible to anyone who has the link.
        </div>
      </header>
    );
  }
}

export default Header;
