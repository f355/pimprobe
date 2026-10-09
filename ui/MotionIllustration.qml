// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

import QtQuick
import "animations/Animations.js" as Animations

Item {
    id: illustration
    required property string clip
    readonly property var metadata: Animations.clips[clip] || {}
    readonly property int phase: (metadata.phases || [0])[sprite.currentFrame] || 0

    AnimatedSprite {
        id: sprite
        objectName: "motionSprite"
        anchors.centerIn: parent
        width: Math.min(parent.width, parent.height * frameWidth / frameHeight)
        height: width * frameHeight / frameWidth
        source: illustration.visible && illustration.clip ? Qt.resolvedUrl("animations/" + illustration.clip + ".png") : ""
        frameWidth: illustration.metadata.width || 320
        frameHeight: illustration.metadata.height || 176
        frameCount: illustration.metadata.frames || 1
        frameRate: illustration.metadata.rate || 12
        loops: AnimatedSprite.Infinite
        interpolate: false
        running: illustration.visible
        onSourceChanged: restart()
    }
}
