// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "./SubscriptionManager.sol";

/// @title Payment Processor
/// @notice Handles payments in native currency and triggers subscription updates.
contract PaymentProcessor is Ownable, ReentrancyGuard, Pausable {
    SubscriptionManager public subscriptionManager;
    
    event PaymentReceived(address indexed user, uint256 amount, uint8 tierId);
    event Withdrawal(address indexed to, uint256 amount);

    constructor(address initialOwner, address _subscriptionManager) Ownable(initialOwner) {
        subscriptionManager = SubscriptionManager(_subscriptionManager);
    }

    /// @notice Update subscription manager address.
    function setSubscriptionManager(address _subscriptionManager) external onlyOwner {
        subscriptionManager = SubscriptionManager(_subscriptionManager);
    }

    /// @notice Purchase a subscription with native currency (ETH/LIT).
    /// @param tierId The tier to purchase.
    function purchaseSubscription(uint8 tierId) external payable nonReentrant whenNotPaused {
        SubscriptionManager.Tier memory tier = subscriptionManager.getTier(tierId);
        
        require(tier.isActive, "Tier not active");
        require(msg.value >= tier.price, "Insufficient payment");

        // Forward call to manager
        subscriptionManager.processSubscription(msg.sender, tierId);

        emit PaymentReceived(msg.sender, msg.value, tierId);

        // Refund excess is optional, keeping it simple: user pays exact or overpays (tip)
    }

    /// @notice Withdraw accumulated funds.
    function withdraw(address payable to, uint256 amount) external onlyOwner nonReentrant {
        require(address(this).balance >= amount, "Insufficient funds");
        (bool sent, ) = to.call{value: amount}("");
        require(sent, "Failed to send Ether");
        emit Withdrawal(to, amount);
    }

    /// @notice Emergency pause.
    function pause() external onlyOwner {
        _pause();
    }

    /// @notice Unpause.
    function unpause() external onlyOwner {
        _unpause();
    }
}
