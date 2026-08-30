/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** undefined - Show modify file md5 log by default, turn off it can save time when there are large video files. */
  "showMD5Log": boolean,
  /** undefined - Enable zip compress and extract with password. */
  "enableZipPassword": boolean,
  /** Zip Password - Password for zip compress and extract. */
  "zipPassword": string,
  /** Enable to Modify the Type of Media Files - Enable to modify the hash of video files. */
  "enableVideo": boolean,
  /** undefined - Enable to modify the hash of audio files. */
  "enableAudio": boolean,
  /** undefined - Enable to modify the hash of image files. */
  "enableImage": boolean
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `modifyHash` command */
  export type ModifyHash = ExtensionPreferences & {}
  /** Preferences accessible in the `restoreHash` command */
  export type RestoreHash = ExtensionPreferences & {}
  /** Preferences accessible in the `zipCompress` command */
  export type ZipCompress = ExtensionPreferences & {}
  /** Preferences accessible in the `zipExtract` command */
  export type ZipExtract = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `modifyHash` command */
  export type ModifyHash = {}
  /** Arguments passed to the `restoreHash` command */
  export type RestoreHash = {}
  /** Arguments passed to the `zipCompress` command */
  export type ZipCompress = {}
  /** Arguments passed to the `zipExtract` command */
  export type ZipExtract = {}
}

