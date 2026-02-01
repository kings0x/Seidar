// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/SubscriptionManager.sol";
import "../src/PaymentProcessor.sol";

contract PaymentProcessorTest is Test {
    SubscriptionManager public manager;
    PaymentProcessor public processor;
    address public owner;
    address public user;
    uint256 public constant TIER_PRICE = 0.01 ether;

    function setUp() public {
        owner = address(this);
        user = address(0x123);
        vm.deal(user, 100 ether);

        manager = new SubscriptionManager(owner);
        processor = new PaymentProcessor(owner, address(manager));
        manager.setPaymentProcessor(address(processor));
    }

    function testPurchaseSubscription() public {
        vm.prank(user);
        processor.purchaseSubscription{value: TIER_PRICE}(1);

        // Check subscription active
        assertTrue(manager.isSubscribed(user, 1));
        
        // Check funds received
        assertEq(address(processor).balance, TIER_PRICE);
    }

    function testInsufficientPayment() public {
        vm.prank(user);
        vm.expectRevert("Insufficient payment");
        processor.purchaseSubscription{value: TIER_PRICE - 1}(1);
    }

    function testWithdraw() public {
        // Fund processor
        vm.prank(user);
        processor.purchaseSubscription{value: TIER_PRICE}(1);

        uint256 initialOwnerBalance = owner.balance;
        
        processor.withdraw(payable(owner), TIER_PRICE);

        assertEq(owner.balance, initialOwnerBalance + TIER_PRICE);
        assertEq(address(processor).balance, 0);
    }

    receive() external payable {}
}
