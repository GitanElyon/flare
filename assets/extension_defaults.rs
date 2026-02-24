ExtensionFileConfig {
    enabled: vec![],
    calculator: CalculatorExtensionConfig {
        trigger: String::from("="),
        replace_symbols: false,
        fancy_numbers: false,
    },
    symbols: SymbolsExtensionConfig {
        trigger: String::from("."),
    },
    help: HelpExtensionConfig {
        trigger: String::from("-"),
    },
    clipboard: ClipboardExtensionConfig {
        trigger: String::from("+"),
        prefer_external_history_tools: true,
    },
}
