const mockServer = require("./mocks/server");
const Message = require('bitcore-message-dash');

exports.login = function(txId, privateKey) {
    var server = new mockServer();
    var signature = new Message(server.challengeMsg).sign(privateKey);
    return server.resolveChallenge(txId, signature); //returns a Promise
}



// exports.login = function () {
//     let self = this;
//     return async function (_u) {
//         return new Promise(function (resolve, reject) {
//             let res = {error: null, result: 'success'};
//             if (
//                 _u &&
//                 has(_u, 'password') &&
//                 (has(_u, 'username') || has(_u, 'email'))
//             ) {
//                 let msg = {
//                     type: "user",
//                     action: "login",
//                     user: _u,
//                     _reqId: uuid.generate.v4()
//                 };
//                 self.emitter.once(msg._reqId, function (data) {
//                     if (data.hasOwnProperty('error') && data.error !== null) {
//                         return resolve(data.message);
//                     } else {
//                         if (has(data,'result') && has(data.result,'username') && has(data.result,'email')) {
//                             self.IS_CONNECTED = true;
//                             self.USER = data.result;
//                         }
//                         return resolve(data);

//                     }
//                 });
//                 self.socket.send(JSON.stringify(msg));
//             }
//             else {
//                 res.error = '100 - Missing Params';
//                 res.result = 'Missing User';
//                 return resolve(res);
//             }
//         });
//     }
// }