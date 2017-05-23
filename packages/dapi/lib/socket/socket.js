class Socket {
    constructor(app){
        app.socket = require('socket.io')();
        app.socket.on('connection', function(client){
            let host = null;
            if(client && client.hasOwnProperty('handshake') && client.handshake.hasOwnProperty('headers') && client.handshake.headers.hasOwnProperty('host')){
                host = client.handshake.headers.host;
            }
            console.log(`Client ${client.conn.id} has connected from ${host}`);

        });
        app.socket.listen(app.config.websocket.port);
        console.log('Websocket started...', app.config.websocket.port);
    }
}
module.exports=Socket;