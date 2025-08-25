# Discord音声合成ボット(discord-parrot)
Discordで使える読み上げBOTです

## ディレクトリ構造
```aiexclude
src/
├── main.rs               // エントリーポイント。基本的にはクライアントの起動と初期化のみ
├── config.rs             // 設定の読み込み(dotenv, configなど)
├── error.rs              // thiserrorを使った独自のエラー型
├── handler.rs            // serenityのイベントハンドラー
├── commands /
│   ├── mod.rs
│   ├── dictionary.rs     // 辞書を管理するコマンド
│   ├── join.rs           // VCに参加するコマンド
│   ├── leave.rs          // VCから切断するコマンド
│   ├── say.rs            // 音声合成してVCで再生するコマンド
│   └── skip.rs           // 音声再生をスキップするコマンド
├── database /
│   ├── mod.rs
│   └── types.rs          // データベースの型
└── voice /
    ├── voicevox /
    │   ├── mod.rs
    │   ├── audio.rs      // VOICEVOXの音声合成
    │   ├── client.rs     // VOICEVOXのクライアント
    │   ├── dictionary.rs // VOICEVOXの辞書の制御
    │   └── format.rs     // VOICEVOX用にDiscordメッセージをフォーマット
    ├── mod.rs
    ├── manager.rs        // VCの接続や制御（Songbird）
    └── playback.rs       // 音声ファイル再生処理
```