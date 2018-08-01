'use strict'
const e = React.createElement;

class CommandsComponent extends React.Component {
    constructor(props) {
        super(props);

        var socket = new WebSocket('ws://' + window.location.host + '/ws/update/');


        this.state = {message_count: 0, update_socket: socket, commands: null};
        var object = this;
        socket.onmessage = function (event) {
            console.log(event.data);
            object.state.message_count += 1;
            object.setState(object.state)
        }

        this.onChange = this.handleChange.bind(this);
    }

    handleChange(event) {
        console.log(event);
    }

    componentDidMount() {
        console.log("TEST");
        fetch("http://127.0.0.1/api/commands/")
        .then(result => {
            return result.json();
        })
            .then(data => {
                let commands = data.map((command) =>
                    <li key={command.match_expr} className = "card blue-grey darken-1">
                        <div className="card-content white-text">
                            <div className="card-title">{command.channel}</div>
                            Expr: <input onChange={this.onChange}type="text" className="" value={command.match_expr} />
                            Command: <input onChange={this.onChange} type="text" className="" value={command.command} />
                        </div>
                   </li>);

                this.setState({commands: commands});
        });
    }


    render() {
        return (
            <ul className="container">{this.state.commands}</ul>
        );
    }
}

const domContainer = document.querySelector('#commands_container');
ReactDOM.render(e(CommandsComponent), domContainer);
