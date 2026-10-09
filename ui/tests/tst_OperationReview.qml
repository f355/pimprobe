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
import QtQuick.Controls
import QtTest
import ".."
import "../controls"
import "../ProbePages.js" as Pages
import "../ReviewPlan.js" as Plan
import "../animations/Animations.js" as Animations

TestCase {
    id: test
    name: "OperationReview"
    when: windowShown
    ScreenClient { id: client }
    Component {
        id: illustrationComponent
        MotionIllustration { width: 320; height: 176; clip: "z-surface" }
    }
    ApplicationWindow {
        id: host
        visible: true
        width: 800
        height: 480
        font.family: "DejaVu Sans"
        color: Theme.page
        OperationReview {
            id: review
            anchors.fill: parent
            anchors.margins: 16
            config: Pages.routine(client.values,"outside",{x:-1,y:-1,z:false},54)
        }
    }
    function init() {
        failOnWarning(/(?!(.*Populating font|.*Sans Serif)).*/);
        review.visible = true;
        review.rotary = false;
        review.config = Pages.routine(client.values,"outside",{x:-1,y:-1,z:false},54);
    }
    function test_edit_and_reset() {
        var input = findChild(review,"review-xSearchDistance");
        mouseClick(input);
        compare(input.selectedText,"10");
        input.editor.typeKey("2");
        input.editor.typeKey("5");
        input.editor.accept();
        compare(input.text,"25");
        compare(review.draft.xSearchDistance,25);
        review.config = Pages.routine(client.values,"outside",{x:1,y:0,z:false},54);
        compare(review.draft.xSearchDistance,10);
        compare(input.text,"10");
    }
    function test_close_discards_unfinished_edit() {
        var input = findChild(review,"review-depth");
        mouseClick(input);
        input.editor.typeKey("9");
        review.visible = false;
        compare(input.text,"5");
        compare(input.editor.target,null);
    }
    function test_distance_options_data() {
        return [
            {tag:"outside corner", family:"outside", selection:{x:-1,y:1,z:false}, keys:["xSearchDistance","ySearchDistance","depth"]},
            {tag:"inside edge", family:"inside", selection:{x:0,y:1,z:false}, keys:["ySearchDistance"]},
            {tag:"inside Z", family:"inside", selection:{x:0,y:0,z:true}, keys:["depth"]},
            {tag:"block", family:"center", selection:"block", keys:["xSearchDistance","ySearchDistance","depth"]},
            {tag:"hole", family:"center", selection:"hole", keys:["xSearchDistance","ySearchDistance"]},
            {tag:"Y ridge", family:"center", selection:"y-ridge", keys:["ySearchDistance","depth"]},
            {tag:"X valley", family:"center", selection:"x-valley", keys:["xSearchDistance"]}
        ];
    }
    function test_distance_options(data) {
        var plan = Plan.describe(Pages.routine(client.values,data.family,data.selection,54),false);
        compare(plan.options.filter(function(option) { return option.unit === "mm" && option.key !== "retract"; })
            .map(function(option) { return option.key; }),data.keys);
        verify(Animations.clips[plan.clip]);
    }
    function test_motion_data() {
        return Object.keys(Animations.clips).map(function(clip) { return {tag:clip}; });
    }
    function test_motion(data) {
        var illustration = createTemporaryObject(illustrationComponent,host.contentItem,{clip:data.tag});
        verify(illustration);
        var sprite = findChild(illustration,"motionSprite");
        verify(waitForPolish(illustration));
        verify(waitForRendering(illustration));
        tryVerify(function() { return sprite.currentFrame > 0; },3000);
        sprite.running = false;
        sprite.currentFrame = 0;
        waitForRendering(illustration);
        var first = grabImage(illustration);
        sprite.currentFrame = 24;
        waitForRendering(illustration);
        var second = grabImage(illustration);
        sprite.currentFrame = 48;
        waitForRendering(illustration);
        var third = grabImage(illustration);
        verify(!first.equals(second) || !first.equals(third),"The rendered probe moves");
    }
}
