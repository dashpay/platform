const has = require('../../util/has.js');
const { uuid } = require('khal');
const ax = require('axios');


exports.search = function(query) {
    return ax.get('http://localhost:4000/graphql/graphiql?' + "query={RootBase" + query.returns + "}");
}

// if(
//     query &&
//     (has(query,'username') ||
//     has(query,'email'))
// ){
//     let msg = {
//         type:"user",
//         action:"search",
//         query:query,
//         _reqId:uuid.generate.v4()
//     };
//     self.emitter.once(msg._reqId, function(data){
//         if(data.hasOwnProperty('error') && data.error==null){
//             return resolve(data);
//         }else{
//             return resolve(data.message);
//         }
//     });
//     self.socket.send(JSON.stringify(msg));
// }
// else{
//     res.error = '100 - Missing Params';
//     res.result = 'Missing Query';
//     return resolve(res);
// }