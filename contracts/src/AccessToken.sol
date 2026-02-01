// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/// @title Access Token (SBT)
/// @notice Soulbound token representing active subscription.
contract AccessToken is ERC721, Ownable {
    uint256 private _nextTokenId;
    address public subscriptionManager;

    error Soulbound();
    error OnlyManager();

    constructor(address initialOwner, address _subscriptionManager) 
        ERC721("LitVM Access Token", "LITACCESS") 
        Ownable(initialOwner) 
    {
        subscriptionManager = _subscriptionManager;
    }

    modifier onlyManager() {
        if (msg.sender != subscriptionManager) revert OnlyManager();
        _;
    }

    function setSubscriptionManager(address _subscriptionManager) external onlyOwner {
        subscriptionManager = _subscriptionManager;
    }

    function mint(address to) external onlyManager returns (uint256) {
        uint256 tokenId = ++_nextTokenId;
        _safeMint(to, tokenId);
        return tokenId;
    }

    function burn(uint256 tokenId) external onlyManager {
        _burn(tokenId);
    }

    /// @notice Disable transfers to make it Soulbound.
    function transferFrom(address, address, uint256) public virtual override {
        revert Soulbound();
    }

    /// @notice Disable transfers to make it Soulbound.
    function safeTransferFrom(address, address, uint256, bytes memory) public virtual override {
        revert Soulbound();
    }
}
