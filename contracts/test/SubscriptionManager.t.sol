// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/SubscriptionManager.sol";
import "../src/PaymentProcessor.sol";

contract SubscriptionManagerTest is Test {
    SubscriptionManager public manager;
    PaymentProcessor public processor;
    address public owner;
    address public user;

    function setUp() public {
        owner = address(this);
        user = address(0x123);

        manager = new SubscriptionManager(owner);
        processor = new PaymentProcessor(owner, address(manager));
        manager.setPaymentProcessor(address(processor));
    }

    function testInitialState() public {
        SubscriptionManager.Tier memory tier = manager.getTier(1);
        assertTrue(tier.isActive);
        assertEq(tier.price, 0.01 ether);
    }

    function testOnlyProcessorCanProcess() public {
        vm.expectRevert(SubscriptionManager.UnauthorizedProcessor.selector);
        manager.processSubscription(user, 1);
    }

    function testProcessSubscriptionViaProcessor() public {
        // Simulate processor call
        vm.prank(address(processor));
        manager.processSubscription(user, 1);

        (uint256 expiry, uint8 tier, bool isActive) = manager.subscriptions(user);
        assertTrue(isActive);
        assertEq(tier, 1);
        assertGt(expiry, block.timestamp);
    }

    function testRenewalExtendsExpiry() public {
        vm.startPrank(address(processor));
        
        manager.processSubscription(user, 1);
        (uint256 expiry1,,) = manager.subscriptions(user);

        manager.processSubscription(user, 1);
        (uint256 expiry2,,) = manager.subscriptions(user);

        assertGt(expiry2, expiry1);
        vm.stopPrank();
    }
}
