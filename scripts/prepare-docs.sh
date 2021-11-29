# Consolidate package docs into one folder for building documentation
cp -r ./packages/js-dapi-client/docs/ ./docs/docs/DAPI-Client
mv ./docs/docs/DAPI-Client/_sidebar.md ./docs/docs/DAPI-Client/Overview.md
cp -r ./packages/js-dpp/docs/ ./docs/docs/Dash-Platform-Protocol
mv ./docs/docs/Dash-Platform-Protocol/_sidebar.md ./docs/docs/Dash-Platform-Protocol/Overview.md
cp -r ./packages/js-dash-sdk/docs/ ./docs/docs/SDK
mv ./docs/docs/SDK/_sidebar.md ./docs/docs/SDK/Overview.md
cp -r ./packages/wallet-lib/docs/ ./docs/docs/Wallet-library
mv ./docs/docs/Wallet-library/_sidebar.md ./docs/docs/Wallet-library/Overview.md
