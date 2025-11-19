# Vex Introduction

Vex is a QEMU auxiliary command-line tool that addresses three core pain points: simplifying complex QEMU startup parameters, lowering the learning and usage barrier for QEMU, and supporting remote distribution of configuration files.

It provides a Docker-like convenient experience, helping users quickly launch full-system simulation environments, suitable for embedded development, firmware development, operating system development, and other scenarios.

# Configuration

## VEX_CONFIG_DIR

Vex supports custom configuration storage location through the `VEX_CONFIG_DIR` environment variable.

- If `VEX_CONFIG_DIR` is set: Vex will save and load configurations from the specified directory
- If `VEX_CONFIG_DIR` is not set: Vex uses `<user_home_dir>/.vex/configs` as the default storage location

This allows for flexible configuration management across different environments and use cases.

# Roadmap

## Phase 1: Building Basic Command Capabilities

Focus on core functionality, implementing local management of QEMU configurations to meet rapid startup requirements.

- Save configuration: `vex save <name> [-y] [-d "desc"] <qemu-bin> [qemu args ...]` —— Save QEMU startup parameters as configurations, eliminating repetitive input. Can overwrite existing names with prompts, -y forces overwrite, and you can add a description for the configuration (optional) using double quotes;
- Rename configuration: `vex rename [-y] [-d "desc"] <old_name> <new_name>` —— Rename saved configurations, maintaining name uniqueness and readability;
- Execute configuration: `vex exec <name>` —— Directly execute saved configurations, one-click QEMU startup;
- View configurations: `vex list` —— List all saved configurations;
- Delete configuration: `vex rm <name>` —— Remove unused configurations, keeping the local environment clean.
- Edit configuration: `vex edit <name>` —— Modify the configuration interactively, with an option to test-run (trial execution) after editing. 

## Phase 2: Implementing Remote Configuration Distribution

Establish configuration sharing channels, supporting team collaboration and cross-environment reuse. Utilize GitHub repositories for resource hosting, allowing users to upload/download images, firmware files, etc., with version management and tag classification support.

- Pull configuration: `vex pull <id/name>:[tag]` —— Pull configurations shared by others from remote sources, quickly reusing mature environments;
- Push configuration: `vex push <id/remote_name>:[tag] <local_name>` —— Push local configurations to remote sources, facilitating team sharing or cross-device usage.

## Phase 3: Supporting Configuration-Associated Images and Firmware, Providing Vex Hub

Complete the "configuration + resources" full pipeline, solving the scattered management issues of images/firmware, and building a complete Vex ecosystem.

Configuration resource association: Support binding specified image files (such as system images) and firmware files in Vex configurations. When executing the exec command, resources are automatically loaded without manual path specification.

Launch Vex Hub: Create an official resource and configuration display platform, providing rich and popular simulation platform environment setup solutions. In the Hub, users can directly obtain complete "configuration + associated resources" packages, with one-click pull and launch of simulation environments, eliminating the need for separate file preparation.
