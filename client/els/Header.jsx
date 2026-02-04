import { Component } from 'preact';

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
        <div className='top-bar'>
          <h1>{this.props.bucketID}</h1>
          {Boolean(this.props.connCount) && (
            <span className='conn-count' title='Number of connected clients'>{this.props.connCount}</span>
          )}
          <input
            type="text"
            id="filter"
            placeholder="Type to filter"
            value={this.props.filterVal}
            onChange={evt => this.props.onFilter(evt.target.value)}
          />
        </div>
        <div className='client-list'>{(this.props.clients || []).join(', ')}</div>
      </header>
    );
  }
}

export default Header;
