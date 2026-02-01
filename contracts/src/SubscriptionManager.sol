// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";

/// @title Subscription Manager
/// @notice Manages subscription lifecycles, tiers, and expiration.
contract SubscriptionManager is Ownable, Pausable {
    struct Subscription {
        uint256 expiry;
        uint8 tier;
        bool isActive;
    }

    struct Tier {
        uint256 price; // Price in Wei or decimals
        uint256 duration; // Duration in seconds
        bool isActive;
    }

    // Mapping from user address to subscription details
    mapping(address => Subscription) public subscriptions;

    // Mapping from tier ID to tier details
    mapping(uint8 => Tier) public tiers;

    // Address authorized to process payments (PaymentProcessor)
    address public paymentProcessor;

    event SubscriptionCreated(address indexed user, uint8 tier, uint256 expiry);
    event SubscriptionRenewed(address indexed user, uint8 tier, uint256 expiry);
    event SubscriptionCancelled(address indexed user);
    event TierUpdated(uint8 indexed tierId, uint256 price, uint256 duration);
    event PaymentProcessorUpdated(address indexed newProcessor);

    error UnauthorizedProcessor();
    error InvalidTier();
    error SubscriptionNotActive();

    modifier onlyProcessor() {
        if (msg.sender != paymentProcessor) revert UnauthorizedProcessor();
        _;
    }

    constructor(address initialOwner) Ownable(initialOwner) {
        // Initialize default tiers
        tiers[1] = Tier(0.01 ether, 30 days, true); // Basic
        tiers[2] = Tier(0.05 ether, 30 days, true); // Premium
    }

    /// @notice Sets the payment processor address.
    function setPaymentProcessor(address _processor) external onlyOwner {
        paymentProcessor = _processor;
        emit PaymentProcessorUpdated(_processor);
    }

    /// @notice Updates or creates a subscription tier.
    function setTier(uint8 tierId, uint256 price, uint256 duration, bool isActive) external onlyOwner {
        tiers[tierId] = Tier(price, duration, isActive);
        emit TierUpdated(tierId, price, duration);
    }

    /// @notice Process a new subscription or renewal. Called by PaymentProcessor.
    /// @param user The subscriber's address.
    /// @param tierId The tier ID.
    function processSubscription(address user, uint8 tierId) external onlyProcessor whenNotPaused {
        Tier memory tier = tiers[tierId];
        if (!tier.isActive) revert InvalidTier();

        Subscription storage sub = subscriptions[user];
        
        uint256 newExpiry;
        if (sub.expiry > block.timestamp && sub.tier == tierId) {
            // Renewal
            newExpiry = sub.expiry + tier.duration;
            emit SubscriptionRenewed(user, tierId, newExpiry);
        } else {
            // New or upgrade/downgrade (reset expiry for simplicity)
            newExpiry = block.timestamp + tier.duration;
            emit SubscriptionCreated(user, tierId, newExpiry);
        }

        sub.expiry = newExpiry;
        sub.tier = tierId;
        sub.isActive = true;
    }

    /// @notice Admin cancellation of a subscription.
    function cancelSubscription(address user) external onlyOwner {
        delete subscriptions[user];
        emit SubscriptionCancelled(user);
    }

    /// @notice Check if a user has a valid subscription for a minimum tier.
    function isSubscribed(address user, uint8 minTier) external view returns (bool) {
        Subscription memory sub = subscriptions[user];
        return sub.isActive && sub.expiry > block.timestamp && sub.tier >= minTier;
    }

    /// @notice Get tier functionality details.
    function getTier(uint8 tierId) external view returns (Tier memory) {
        return tiers[tierId];
    }
}
