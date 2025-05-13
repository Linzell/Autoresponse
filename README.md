# Autoresponse: Unified Service Notifications Manager

A desktop application that centralizes notifications and actions from various services with automated response capabilities using local AI models. Built with Tauri V2, React, and TypeScript.

## Quick Start

1. Clone the repository:
```bash
git clone https://github.com/yourusername/autoresponse.git
cd autoresponse
```

2. Install dependencies:
```bash
bun install
```

3. Run development environment:
```bash
bun tauri dev
```

4. Build for production:
```bash
bun tauri build
```

## Documentation

Detailed documentation can be found in the following locations:

- [Backend Documentation](src-tauri/docs/BACKEND.md) - Architecture, domain model, and backend integration
- [API Documentation](src-tauri/docs/API.md) - Complete API reference for frontend developers

## Development Prerequisites

### Required Software
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/) (Latest stable)

## Project Structure

```
autoresponse/
├── src-tauri/                 # Rust backend code
│   ├── src/
│   │   ├── application/      # Use cases and business logic
│   │   ├── domain/          # Core domain entities and interfaces
│   │   ├── infrastructure/  # External implementations
│   │   └── presentation/    # API controllers and DTOs
│   └── tauri.conf.json      # Tauri configuration
├── src/                      # React frontend code
│   ├── components/          # Reusable UI components
│   ├── features/           # Feature-specific modules
│   ├── hooks/             # Custom React hooks
│   ├── services/          # API service integrations
│   └── utils/             # Utility functions
└── package.json
```

## Quality Assurance

- Unit test coverage: 100%
- Integration test coverage: 85%
- E2E test coverage: 70%
- No TypeScript/Rust compiler errors
- No ESLint/Clippy warnings
- WCAG 2.1 AA compliance

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

For support, please:
1. Check the documentation in the `docs` directory
2. Open an issue in the GitHub repository
3. Contact the maintainers directly
