'use strict'

const e = React.createElement;

class StatsComponent extends React.Component {
    constructor(props) {
        super(props);

        var socket = new WebSocket('ws://'+window.location.host + '/ws/update/');


        this.state = {message_count: 0, update_socket: socket};
        var object = this;
        socket.onmessage = function (event) {
            console.log(event.data);
            object.state.message_count += 1;
            object.setState(object.state)
        }
    };

    render() {
        return 'Message Count: ' + this.state.message_count;
    }
}

const domContainer = document.querySelector('#stats_container');
ReactDOM.render(e(StatsComponent), domContainer);

