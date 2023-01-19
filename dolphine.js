const dolphine = {
      _socketAddr: "127.0.0.1",
      _socketPort: "8080",
      _responses: [],
      _pollingDelay: 50, // ms
      _current_id: 0,

      _init: function() {
            this._websocket = new WebSocket(`ws://${this._socketAddr}:${this._socketPort}`)
            this._websocket.onmessage = (message) => {
                  let data = JSON.parse(message.data);
                  if (data.actiontype == 0) { // run when server response comes back
                        this._responses.push(data);
                  } else if (data.actiontype == 2) { // run when server (rust) registers a rust function in javascript
                        this._rustRegister(data)
                  } else {
                        return // impossible to reach
                  }
            }
      },

      _genId: () => {
            let id = this._current_id;
            this._current_id += 1;
            return String(id);
      },

      _rustRegister: function(data) {
            let registerName = data.register_as || data.function; // string
            let functionName = data.function;
            let argLength = data.args; // array

            this[registerName] = async (...args) => {
                  if (args.length != argLength) {
                        throw {name: "IncorrectNumArgsError", message: `Incorrect number of arguments passed to the function. Expected: ${argLength}. Found: ${args.length}`};
                  }
                  let id = this._genId();
                  let data = {
                        args: JSON.stringify(args),
                        id,
                        function: functionName,
                        actiontype: 1,
                  }
                  this._websocket.send(JSON.stringify(data));
                  let reply;
                  while (true) {
                        for (let data of this._responses) {
                              if (data.id == id) {
                                    reply = data;
                                    this._responses.pop(this._responses.indexOf(reply));
                                    break;
                              }
                        }
                        if (reply) {
                              break;
                        }
                        await new Promise(r => setTimeout(r, 50));
                  }
                  if (reply.success != true) {
                        throw {name: "RustFunctionFailed", message: `The function on the rust side failed: ${reply.data}`}
                  }
                  return reply.data;
            }
      },
};

dolphine._init()