const axios = require('axios');

exports.send = function(rawtx) {
    let self = this;
    let demo = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff13033911030e2f5032506f6f6c2d74444153482fffffffff0479e36542000000001976a914f0adf747fe902643c66eb6508305ba2e1564567a88ac40230e43000000001976a914f9ee3a27ef832846cf4ad40fe95351effe4a485d88acc73fa800000000004341047559d13c3f81b1fadbd8dd03e4b5a1c73b05e2b980e00d467aa9440b29c7de23664dde6428d75cafed22ae4f0d302e26c5c5a5dd4d3e1b796d7281bdc9430f35ac00000000000000002a6a283662876fa09d54098cc66c0a041667270a582b0ea19428ed975b5b5dfb3bca79000000000200000000000000"
    return async function(){
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `https://dev-test.dash.org/insight-api-dash/tx/send`;  //hard coded for now due to version issue
            return axios
              .post(url, {rawtx: rawtx || demo})
              .then(function(response){
                // console.log("!!!", response.data)
                return resolve(response.data);
              })
              .catch(function(error){
                if(error){
                    // console.log("!!!", error.response.data)
                    console.log(url, error.response.data)
                    console.error(`An error was triggered while sending transaction.`);
                    return resolve(false);
                }
            });
        });
    }
}
