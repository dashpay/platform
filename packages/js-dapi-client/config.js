const Config = {
    DISCOVER: {
        INSIGHT_SEEDS: [
            /*{
                protocol:"https",
                path:'/api',
                base:"insight.dash.siampm.com",
                port: 80,
                fullPath:"https://insight.dash.siampm.com/api"
            }*/
        ],
        SOCKET_SEEDS: {
            /*ipv6: [
                {uri: "::", port: 80}
            ]*/
        }
    },
    ROUTER:{
        port:80,
        host:'::'//Allow ipv6
    },
    debug:true,
    verbose:false,
    warnings:true,
    errors:true
};
module.exports = Config;