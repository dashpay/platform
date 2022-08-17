const path = require('path');
const webpack = require('webpack');

module.exports = {
    entry: './mod.js',
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: 'index.js',
        library: {
            type: 'umd'
        },
        publicPath: '',
        // This is needed to prevent ReferenceError: self is not defined,
        // as webpack names global object "self" for some reason
        globalObject: 'this'
    },
    plugins: [
        // // Have this example work in Edge which doesn't ship `TextEncoder` or
        // // `TextDecoder` at this time.
        // new webpack.ProvidePlugin({
        //     TextDecoder: ['text-encoding', 'TextDecoder'],
        //     TextEncoder: ['text-encoding', 'TextEncoder']
        // })
    ],
    mode: 'development',
    // experiments: {
    //     asyncWebAssembly: true
    // },
    resolve: {
        extensions: ['.wasm']
    },
    module: {
        rules: [{
            test: /\.wasm$/,
            type: "asset/inline",
        }],
    }
};
