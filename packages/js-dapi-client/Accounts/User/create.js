const has = require('../../util/has.js');
const {uuid}=require('khal');
const ax = require('axios');

exports.create = function() {
    let self = this;
    return async function (_u) {
        return new Promise(function (resolve, reject) {
            let res = {error: null, result: 'success'};

            // comment out client side valdiation for now
            // if(true ||
            //     _u &&
            //     has(_u,'username') &&
            //     has(_u,'password') &&
            //     has(_u,'email')
            // ){
            //     let msg = {
            //         type:"user",
            //         action:"create",
            //         user:_u,
            //         _reqId:uuid.generate.v4()
            //     };
            //
                // self.emitter.once(msg._reqId, function(data){
                //     if(data.hasOwnProperty('error') && data.error==null){
                //         return resolve(data.message);
                //     }else{
                //         return resolve(data.message);
                //     }
                // });
                // self.socket.send(JSON.stringify(msg));
            // }
            // else{
            //     res.error = '100 - Missing Params';
            //     res.result = 'Missing User';
            //     return resolve(res);
            // }

            // console.log(_u.params)
            //     console.log(_u.returns)
            //     console.log({"query":"mutation{addRootBase(obj:"+ _u.params+")"+_u.returns+"}"})
            ax.post('http://localhost:4000/graphql/graphiql',
                {"query":"mutation{add"+_u.base+"(obj:"+ _u.params+")"+_u.returns+"}"})
                .then(function (response) {
                    return resolve(response.data.data)
                })
                .catch(function (error) {
                    console.log(error.response.data.errors);
                });
        });
    }
}