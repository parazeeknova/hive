// SPDX-License-Identifier: MIT
pragma solidity ^0.8.38;
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

// TEST FIXTURE ONLY. Anyone can mint. Never deploy or fund this on a public chain.
contract HiveTestUSDC is ERC20, EIP712 {
    mapping(address => mapping(bytes32 => bool)) public authorizationState;
    bytes32 private constant AUTHORIZATION_TYPEHASH = keccak256("TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)");
    event AuthorizationUsed(address indexed authorizer, bytes32 indexed nonce);
    constructor() ERC20("Hive Local Test USD Coin", "USDC") EIP712("USDC", "2") {}
    function decimals() public pure override returns (uint8) { return 6; }
    function mint(address recipient, uint256 value) external { _mint(recipient, value); }
    function transferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce,uint8 v,bytes32 r,bytes32 s) external {
        _authorizedTransfer(from,to,value,validAfter,validBefore,nonce,abi.encodePacked(r,s,v));
    }
    function transferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce,bytes memory signature) external {
        _authorizedTransfer(from,to,value,validAfter,validBefore,nonce,signature);
    }
    function _authorizedTransfer(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce,bytes memory signature) private {
        require(block.timestamp > validAfter && block.timestamp < validBefore, "authorization time");
        require(!authorizationState[from][nonce], "authorization used");
        bytes32 digest=_hashTypedDataV4(keccak256(abi.encode(AUTHORIZATION_TYPEHASH,from,to,value,validAfter,validBefore,nonce)));
        require(ECDSA.recover(digest,signature)==from,"authorization signer");
        authorizationState[from][nonce]=true;
        emit AuthorizationUsed(from,nonce);
        _transfer(from,to,value);
    }
}
