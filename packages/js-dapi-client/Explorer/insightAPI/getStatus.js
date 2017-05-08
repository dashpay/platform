const _fetch = require('../../util/fetcher.js')._fetch;
const axios = require('axios');

exports.getStatus = function() {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `${getInsightURI}/status`;
            return axios
                .get(url)
                .then(function(response){
                    if(response.hasOwnProperty('data'))
                        return resolve(response.data);
                    else
                        return resolve(null);
                })
                .catch(function(error){
                    if(error){
                        //TODO : Signaling + removal feat
                        console.error(`An error was triggered while fetching candidate ${getInsightCandidate.idx} - signaling and removing from list`);
                        return resolve(false);
                    }
                });
        });
    }
}

exports.getMNlist = function() {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = 'http://test-insight.dev.dash.org/insight-api-dash/status?q=getMNlist';
            let protocol = url.split(':')[0];
            let path = url.split(/(\/+)/)[4]


            console.log('./Explorer/insightAPI/getStatus', `replace with this when PR done: ${getInsightURI}/status?q=getMNlist`); //!!!! replace with this when PR done
            _fetch({type: "GET", url: url}, function (err, data){
                if(err){
                    console.error(`An error was triggered while fetching candidate ${getInsightCandidate.idx} - signaling and removing from list`);
                    //TODO: Do this thing!
                    return resolve(false);
                }
                var parsed = data.slice(1,10).map((mn)=>{
                  return {
                  protocol: protocol,
                  path: `/${path}`,
                  base: mn.IP.split(':')[0],
                  port: mn.IP.split(':')[1],
                  }
                })
                return resolve(parsed);
            });
        });
    }
}
