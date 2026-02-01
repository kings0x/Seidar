// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../src/SubscriptionManager.sol";
import "../src/PaymentProcessor.sol";
import "../src/AccessToken.sol";

contract DeployScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy SubscriptionManager
        SubscriptionManager manager = new SubscriptionManager(deployer);
        console.log("SubscriptionManager deployed at:", address(manager));

        // 2. Deploy PaymentProcessor
        PaymentProcessor processor = new PaymentProcessor(deployer, address(manager));
        console.log("PaymentProcessor deployed at:", address(processor));

        // 3. Deploy AccessToken
        AccessToken token = new AccessToken(deployer, address(manager));
        console.log("AccessToken deployed at:", address(token));

        // 4. Wire everything up
        manager.setPaymentProcessor(address(processor));
        token.setSubscriptionManager(address(manager)); // If AccessToken logic requires it (currently doesn't use it but good for future)
        
        vm.stopBroadcast();
    }
}
