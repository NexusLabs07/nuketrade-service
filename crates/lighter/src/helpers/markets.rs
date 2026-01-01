#[derive(Debug)]
pub struct Market {
    pub symbol: &'static str,
    pub market_index: u32,
}
pub const MARKETS: &[Market] = &[
    Market {
        symbol: "USDCHF",
        market_index: 99,
    },
    Market {
        symbol: "BERA",
        market_index: 20,
    },
    Market {
        symbol: "SEI",
        market_index: 32,
    },
    Market {
        symbol: "KAITO",
        market_index: 33,
    },
    Market {
        symbol: "JUP",
        market_index: 26,
    },
    Market {
        symbol: "APT",
        market_index: 31,
    },
    Market {
        symbol: "IP",
        market_index: 34,
    },
    Market {
        symbol: "LTC",
        market_index: 35,
    },
    Market {
        symbol: "CRV",
        market_index: 36,
    },
    Market {
        symbol: "EIGEN",
        market_index: 49,
    },
    Market {
        symbol: "USELESS",
        market_index: 66,
    },
    Market {
        symbol: "ICP",
        market_index: 102,
    },
    Market {
        symbol: "ZEC",
        market_index: 90,
    },
    Market {
        symbol: "MON",
        market_index: 91,
    },
    Market {
        symbol: "AMZN",
        market_index: 114,
    },
    Market {
        symbol: "USDCAD",
        market_index: 100,
    },
    Market {
        symbol: "XMR",
        market_index: 77,
    },
    Market {
        symbol: "ONDO",
        market_index: 38,
    },
    Market {
        symbol: "ETH",
        market_index: 0,
    },
    Market {
        symbol: "BTC",
        market_index: 1,
    },
    Market {
        symbol: "GOOGL",
        market_index: 116,
    },
    Market {
        symbol: "PLTR",
        market_index: 111,
    },
    Market {
        symbol: "HOOD",
        market_index: 108,
    },
    Market {
        symbol: "SPX",
        market_index: 42,
    },
    Market {
        symbol: "TRX",
        market_index: 43,
    },
    Market {
        symbol: "SYRUP",
        market_index: 44,
    },
    Market {
        symbol: "PUMP",
        market_index: 45,
    },
    Market {
        symbol: "LDO",
        market_index: 46,
    },
    Market {
        symbol: "PENGU",
        market_index: 47,
    },
    Market {
        symbol: "OP",
        market_index: 55,
    },
    Market {
        symbol: "PROVE",
        market_index: 57,
    },
    Market {
        symbol: "GMX",
        market_index: 61,
    },
    Market {
        symbol: "DOLO",
        market_index: 75,
    },
    Market {
        symbol: "MYX",
        market_index: 80,
    },
    Market {
        symbol: "1000TOSHI",
        market_index: 81,
    },
    Market {
        symbol: "MEGA",
        market_index: 94,
    },
    Market {
        symbol: "EURUSD",
        market_index: 96,
    },
    Market {
        symbol: "GBPUSD",
        market_index: 97,
    },
    Market {
        symbol: "FIL",
        market_index: 103,
    },
    Market {
        symbol: "STRK",
        market_index: 104,
    },
    Market {
        symbol: "USDKRW",
        market_index: 105,
    },
    Market {
        symbol: "AVAX",
        market_index: 9,
    },
    Market {
        symbol: "MKR",
        market_index: 28,
    },
    Market {
        symbol: "PENDLE",
        market_index: 37,
    },
    Market {
        symbol: "S",
        market_index: 40,
    },
    Market {
        symbol: "XLM",
        market_index: 119,
    },
    Market {
        symbol: "NMR",
        market_index: 74,
    },
    Market {
        symbol: "DOGE",
        market_index: 3,
    },
    Market {
        symbol: "1000PEPE",
        market_index: 4,
    },
    Market {
        symbol: "1000FLOKI",
        market_index: 19,
    },
    Market {
        symbol: "GRASS",
        market_index: 52,
    },
    Market {
        symbol: "LAUNCHCOIN",
        market_index: 54,
    },
    Market {
        symbol: "PAXG",
        market_index: 48,
    },
    Market {
        symbol: "ARB",
        market_index: 50,
    },
    Market {
        symbol: "YZY",
        market_index: 70,
    },
    Market {
        symbol: "XPL",
        market_index: 71,
    },
    Market {
        symbol: "ASTER",
        market_index: 83,
    },
    Market {
        symbol: "APEX",
        market_index: 86,
    },
    Market {
        symbol: "FF",
        market_index: 87,
    },
    Market {
        symbol: "POL",
        market_index: 14,
    },
    Market {
        symbol: "DYDX",
        market_index: 62,
    },
    Market {
        symbol: "MNT",
        market_index: 63,
    },
    Market {
        symbol: "MORPHO",
        market_index: 68,
    },
    Market {
        symbol: "VVV",
        market_index: 69,
    },
    Market {
        symbol: "2Z",
        market_index: 88,
    },
    Market {
        symbol: "NEAR",
        market_index: 10,
    },
    Market {
        symbol: "TRUMP",
        market_index: 15,
    },
    Market {
        symbol: "1000BONK",
        market_index: 18,
    },
    Market {
        symbol: "FARTCOIN",
        market_index: 21,
    },
    Market {
        symbol: "AI16Z",
        market_index: 22,
    },
    Market {
        symbol: "ZK",
        market_index: 56,
    },
    Market {
        symbol: "ADA",
        market_index: 39,
    },
    Market {
        symbol: "CRO",
        market_index: 73,
    },
    Market {
        symbol: "STBL",
        market_index: 85,
    },
    Market {
        symbol: "NZDUSD",
        market_index: 107,
    },
    Market {
        symbol: "MET",
        market_index: 95,
    },
    Market {
        symbol: "BCH",
        market_index: 58,
    },
    Market {
        symbol: "HBAR",
        market_index: 59,
    },
    Market {
        symbol: "ZRO",
        market_index: 60,
    },
    Market {
        symbol: "PYTH",
        market_index: 78,
    },
    Market {
        symbol: "META",
        market_index: 117,
    },
    Market {
        symbol: "NVDA",
        market_index: 110,
    },
    Market {
        symbol: "LINEA",
        market_index: 76,
    },
    Market {
        symbol: "TSLA",
        market_index: 112,
    },
    Market {
        symbol: "AAPL",
        market_index: 113,
    },
    Market {
        symbol: "CC",
        market_index: 101,
    },
    Market {
        symbol: "POPCAT",
        market_index: 23,
    },
    Market {
        symbol: "HYPE",
        market_index: 24,
    },
    Market {
        symbol: "BNB",
        market_index: 25,
    },
    Market {
        symbol: "WIF",
        market_index: 5,
    },
    Market {
        symbol: "WLD",
        market_index: 6,
    },
    Market {
        symbol: "XRP",
        market_index: 7,
    },
    Market {
        symbol: "TON",
        market_index: 12,
    },
    Market {
        symbol: "SUI",
        market_index: 16,
    },
    Market {
        symbol: "STABLE",
        market_index: 118,
    },
    Market {
        symbol: "ENA",
        market_index: 29,
    },
    Market {
        symbol: "0G",
        market_index: 84,
    },
    Market {
        symbol: "AERO",
        market_index: 65,
    },
    Market {
        symbol: "WLFI",
        market_index: 72,
    },
    Market {
        symbol: "EDEN",
        market_index: 89,
    },
    Market {
        symbol: "DOT",
        market_index: 11,
    },
    Market {
        symbol: "UNI",
        market_index: 30,
    },
    Market {
        symbol: "AVNT",
        market_index: 82,
    },
    Market {
        symbol: "ETH/USDC",
        market_index: 2048,
    },
    Market {
        symbol: "LIT/USDC",
        market_index: 2049,
    },
    Market {
        symbol: "SKY",
        market_index: 79,
    },
    Market {
        symbol: "LINK",
        market_index: 8,
    },
    Market {
        symbol: "RESOLV",
        market_index: 51,
    },
    Market {
        symbol: "ZORA",
        market_index: 53,
    },
];
