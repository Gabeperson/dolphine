const dolphine = {
      _socketAddr: "127.0.0.1",
      _socketPort: "8080",
      _promises: [],
      _currentId: 0,

      _init: function() {
            this._websocket = new WebSocket(`ws://${this._socketAddr}:${this._socketPort}`)
            this._websocket.onmessage = (message) => {
                  let data = JSON.parse(message.data);
                  if (data.actiontype == 0) { // run when server response comes back
                        this._rustReturn(data);
                  } else if (data.actiontype == 2) { // run when server (rust) registers a rust function in javascript
                        this._rustRegister(data);
                  } else {
                        return // impossible to reach
                  }
            }
      },

      _genId: function() {
            let id = this._currentId;
            this._currentId += 1;
            return String(id);
      },

      _rustReturn: function(receivedData) {
            let promise;
            for (let data of this._promises) {
                  if (data.id == receivedData.id) {
                        promise = data;
                        this._promises.pop(this._promises.indexOf(promise));
                        break;
                  }
            }
            if (receivedData.success != true) {
                  console.log(receivedData);
                  promise.reject(receivedData.data)
                  return;
            }
            promise.resolve(receivedData.data);
            return;
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
                  let resolve;
                  let reject;
                  let promise = new Promise(function(insideResolve, insideReject) {
                        resolve = insideResolve;
                        reject = insideReject;
                  });
                  this._promises.push({id, resolve, reject});
                  this._websocket.send(JSON.stringify(data));
                  return promise;
            }
      },
};

dolphine._init()