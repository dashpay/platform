class Handlers {
    constructor(app) {
        return {
            getHello: function(req, res) {
                res.send('Hello world');
            },
            getBlockCount: function(req, res) {
                app.rpc.getBlockCount().then(function(result) {
                    res.send(result);
                });
            },
            getInfo: function(req, res) {
                app.rpc.getInfo().then(function(result) {
                    res.send(result);
                });
            }
        }
    }
}
module.exports = Handlers;